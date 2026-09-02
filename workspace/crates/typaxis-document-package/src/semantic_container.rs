//! Private contract-1.4 carrier for MI4 staging. The public strict decoder and
//! current aliases remain on contract 1.3 until MI4-13.

use crate::{
    DocumentPackageDecodePolicy, JsonPreflightError, StrictJsonPreflight,
    WireAdvancedPageMasterSet, WirePageMasterSet, WireSourceSpan,
};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Number, Value};
use std::collections::BTreeSet;
use std::fmt;
use typaxis_core::{push_jcs_string, sha256, ValidatedResourceLimits, JSON_SAFE_INTEGER_MAX};

pub const STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT: &str = "typaxis.contract/1.4";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireStagingSemanticContainerKind {
    Result,
    Proof,
    Exercise,
}

impl WireStagingSemanticContainerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Result => "result",
            Self::Proof => "proof",
            Self::Exercise => "exercise",
        }
    }
}

impl<'de> Deserialize<'de> for WireStagingSemanticContainerKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "result" => Ok(Self::Result),
            "proof" => Ok(Self::Proof),
            "exercise" => Ok(Self::Exercise),
            _ => Err(de::Error::custom(
                "unknown semantic_container semantic_kind",
            )),
        }
    }
}

impl Serialize for WireStagingSemanticContainerKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireImageMediaType {
    Png,
    SvgSafe1,
    SvgSafe2,
}

impl WireImageMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::SvgSafe1 => "svg-safe-1",
            Self::SvgSafe2 => "svg-safe-2",
        }
    }
}

impl<'de> Deserialize<'de> for WireImageMediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "png" => Ok(Self::Png),
            "svg-safe-1" => Ok(Self::SvgSafe1),
            "svg-safe-2" => Ok(Self::SvgSafe2),
            _ => Err(de::Error::custom("unknown image media_type")),
        }
    }
}

impl Serialize for WireImageMediaType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireFontMediaType {
    SfntTrueTypeGlyf,
    TtcTrueTypeGlyf,
}

impl WireFontMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SfntTrueTypeGlyf => "sfnt-truetype-glyf",
            Self::TtcTrueTypeGlyf => "ttc-truetype-glyf",
        }
    }
}

impl<'de> Deserialize<'de> for WireFontMediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "sfnt-truetype-glyf" => Ok(Self::SfntTrueTypeGlyf),
            "ttc-truetype-glyf" => Ok(Self::TtcTrueTypeGlyf),
            _ => Err(de::Error::custom("unknown font media_type")),
        }
    }
}

impl Serialize for WireFontMediaType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WireStagingM4ReferenceFormat {
    Text,
    Page,
    Number,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireStagingM4LinkTarget {
    Internal { anchor_id: String },
    Uri { uri: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingTextSpan {
    pub text_id: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingMathSource {
    pub language: String,
    pub version: String,
    pub text_span: WireStagingTextSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireVectorProvenance {
    pub engine_id: String,
    pub engine_version: String,
    pub rules_version: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WirePrecomposedVectorViewport {
    pub height: i64,
    pub width: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WirePrecomposedVectorMetrics {
    pub advance: i64,
    pub ascent: i64,
    pub baseline: i64,
    pub descent: i64,
    pub origin_x: i64,
    pub viewport: WirePrecomposedVectorViewport,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WirePrecomposedVectorSpacing {
    pub after: i64,
    pub before: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WirePrecomposedVectorSourceTex {
    pub text_span: WireStagingTextSpan,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WirePrecomposedVectorEquationNumber {
    pub minimum_gap: i64,
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub text_span: WireStagingTextSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireDocumentMetadata {
    pub author: Option<String>,
    pub created: Option<String>,
    pub identifier: Option<String>,
    pub keywords: Vec<String>,
    pub modified: Option<String>,
    pub subject: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WireOutlineSourceKind {
    Heading,
    SemanticContainer,
}

impl WireOutlineSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::SemanticContainer => "semantic_container",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireOutlineEntry {
    pub destination: String,
    pub label: String,
    pub level: u8,
    pub outline_id: u32,
    pub parent_outline_id: Option<u32>,
    pub source_kind: WireOutlineSourceKind,
    pub source_node_id: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireDocumentOutline {
    pub entries: Vec<WireOutlineEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireStagingM4Inline {
    Text {
        node_id: u32,
        span: WireStagingSourceSpan,
        text_span: WireStagingTextSpan,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    InlineMath {
        node_id: u32,
        span: WireStagingSourceSpan,
        math_source: WireStagingMathSource,
        speech: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    InlineVector {
        actual_text: Option<String>,
        alt: String,
        image_id: u32,
        metrics: WirePrecomposedVectorMetrics,
        node_id: u32,
        spacing: WirePrecomposedVectorSpacing,
        span: WireStagingSourceSpan,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    MathVector {
        actual_text: Option<String>,
        alt: String,
        image_id: u32,
        metrics: WirePrecomposedVectorMetrics,
        node_id: u32,
        source_tex: WirePrecomposedVectorSourceTex,
        spacing: WirePrecomposedVectorSpacing,
        span: WireStagingSourceSpan,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Emphasis {
        node_id: u32,
        span: WireStagingSourceSpan,
        children: Vec<WireStagingM4Inline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Strong {
        node_id: u32,
        span: WireStagingSourceSpan,
        children: Vec<WireStagingM4Inline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Link {
        node_id: u32,
        span: WireStagingSourceSpan,
        target: WireStagingM4LinkTarget,
        children: Vec<WireStagingM4Inline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Anchor {
        node_id: u32,
        span: WireStagingSourceSpan,
        anchor_id: String,
    },
    Reference {
        node_id: u32,
        span: WireStagingSourceSpan,
        target: String,
        format: WireStagingM4ReferenceFormat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    FootnoteReference {
        node_id: u32,
        span: WireStagingSourceSpan,
        footnote_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    SoftBreak {
        node_id: u32,
        span: WireStagingSourceSpan,
    },
    HardBreak {
        node_id: u32,
        span: WireStagingSourceSpan,
    },
}

impl WireStagingM4Inline {
    pub const fn node_id(&self) -> u32 {
        match self {
            Self::Text { node_id, .. }
            | Self::InlineMath { node_id, .. }
            | Self::InlineVector { node_id, .. }
            | Self::MathVector { node_id, .. }
            | Self::Emphasis { node_id, .. }
            | Self::Strong { node_id, .. }
            | Self::Link { node_id, .. }
            | Self::Anchor { node_id, .. }
            | Self::Reference { node_id, .. }
            | Self::FootnoteReference { node_id, .. }
            | Self::SoftBreak { node_id, .. }
            | Self::HardBreak { node_id, .. } => *node_id,
        }
    }

    pub const fn span(&self) -> WireStagingSourceSpan {
        match self {
            Self::Text { span, .. }
            | Self::InlineMath { span, .. }
            | Self::InlineVector { span, .. }
            | Self::MathVector { span, .. }
            | Self::Emphasis { span, .. }
            | Self::Strong { span, .. }
            | Self::Link { span, .. }
            | Self::Anchor { span, .. }
            | Self::Reference { span, .. }
            | Self::FootnoteReference { span, .. }
            | Self::SoftBreak { span, .. }
            | Self::HardBreak { span, .. } => *span,
        }
    }

    pub fn language(&self) -> Option<&str> {
        match self {
            Self::Text { language, .. }
            | Self::InlineMath { language, .. }
            | Self::InlineVector { language, .. }
            | Self::MathVector { language, .. }
            | Self::Emphasis { language, .. }
            | Self::Strong { language, .. }
            | Self::Link { language, .. }
            | Self::Reference { language, .. }
            | Self::FootnoteReference { language, .. } => language.as_deref(),
            Self::Anchor { .. } | Self::SoftBreak { .. } | Self::HardBreak { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4ListItem {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub blocks: Vec<WireStagingM4Block>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4TableCell {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub colspan: u16,
    pub rowspan: u16,
    pub blocks: Vec<WireStagingM4Block>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4TableRow {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub cells: Vec<WireStagingM4TableCell>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireStagingM4Block {
    Paragraph {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        children: Vec<WireStagingM4Inline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Heading {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        level: u8,
        anchor_id: Option<String>,
        children: Vec<WireStagingM4Inline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    List {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        ordered: bool,
        start: Option<u32>,
        items: Vec<WireStagingM4ListItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Table {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        columns: Vec<Value>,
        head: Vec<WireStagingM4TableRow>,
        body: Vec<WireStagingM4TableRow>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Figure {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        image_id: u32,
        placement: String,
        alt: String,
        caption: Vec<WireStagingM4Block>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    PageBreak {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
    },
    DisplayMath {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        math_source: WireStagingMathSource,
        speech: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    VectorFigure {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        image_id: u32,
        viewport: WirePrecomposedVectorViewport,
        alt: String,
        caption: Vec<WireStagingM4Block>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    MathVectorBlock {
        actual_text: Option<String>,
        alt: String,
        classes: Vec<String>,
        equation_number: Option<WirePrecomposedVectorEquationNumber>,
        image_id: u32,
        metrics: WirePrecomposedVectorMetrics,
        node_id: u32,
        source_tex: WirePrecomposedVectorSourceTex,
        span: WireStagingSourceSpan,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    SemanticContainer {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        semantic_kind: WireStagingSemanticContainerKind,
        anchor_id: Option<String>,
        blocks: Vec<WireStagingM4Block>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
}

impl WireStagingM4Block {
    pub const fn node_id(&self) -> u32 {
        match self {
            Self::Paragraph { node_id, .. }
            | Self::Heading { node_id, .. }
            | Self::List { node_id, .. }
            | Self::Table { node_id, .. }
            | Self::Figure { node_id, .. }
            | Self::PageBreak { node_id, .. }
            | Self::DisplayMath { node_id, .. }
            | Self::VectorFigure { node_id, .. }
            | Self::MathVectorBlock { node_id, .. }
            | Self::SemanticContainer { node_id, .. } => *node_id,
        }
    }

    pub const fn span(&self) -> WireSourceSpan {
        let span = match self {
            Self::Paragraph { span, .. }
            | Self::Heading { span, .. }
            | Self::List { span, .. }
            | Self::Table { span, .. }
            | Self::Figure { span, .. }
            | Self::PageBreak { span, .. }
            | Self::DisplayMath { span, .. }
            | Self::VectorFigure { span, .. }
            | Self::MathVectorBlock { span, .. }
            | Self::SemanticContainer { span, .. } => span,
        };
        span.into_public()
    }

    pub fn classes(&self) -> &[String] {
        match self {
            Self::Paragraph { classes, .. }
            | Self::Heading { classes, .. }
            | Self::List { classes, .. }
            | Self::Table { classes, .. }
            | Self::Figure { classes, .. }
            | Self::PageBreak { classes, .. }
            | Self::DisplayMath { classes, .. }
            | Self::VectorFigure { classes, .. }
            | Self::MathVectorBlock { classes, .. }
            | Self::SemanticContainer { classes, .. } => classes,
        }
    }

    pub fn language(&self) -> Option<&str> {
        match self {
            Self::Paragraph { language, .. }
            | Self::Heading { language, .. }
            | Self::List { language, .. }
            | Self::Table { language, .. }
            | Self::Figure { language, .. }
            | Self::DisplayMath { language, .. }
            | Self::VectorFigure { language, .. }
            | Self::MathVectorBlock { language, .. }
            | Self::SemanticContainer { language, .. } => language.as_deref(),
            Self::PageBreak { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingSourceSpan {
    pub source_id: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

impl WireStagingSourceSpan {
    pub const fn into_public(self) -> WireSourceSpan {
        WireSourceSpan {
            source_id: self.source_id,
            start_byte: self.start_byte,
            end_byte: self.end_byte,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4Footnote {
    pub footnote_id: String,
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub blocks: Vec<WireStagingM4Block>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4Document {
    pub node_id: u32,
    pub blocks: Vec<WireStagingM4Block>,
    pub footnotes: Vec<WireStagingM4Footnote>,
    pub language: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4FontFace {
    pub font_face_id: u32,
    pub family: String,
    pub uri: String,
    pub face_index: u32,
    pub expected_sha256: Option<String>,
    pub media_type: WireFontMediaType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4Image {
    pub image_id: u32,
    pub uri: String,
    pub expected_sha256: Option<String>,
    pub media_type: WireImageMediaType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_provenance: Option<WireVectorProvenance>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4ResourceCatalog {
    pub font_faces: Vec<WireStagingM4FontFace>,
    pub images: Vec<WireStagingM4Image>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4Source {
    pub source_id: u32,
    pub uri: String,
    pub utf8_byte_length: u32,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingByteRange {
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WireStagingTextMapKind {
    Identity,
    Replacement,
    Inserted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingTextMapSegment {
    pub text_range: WireStagingByteRange,
    pub kind: WireStagingTextMapKind,
    pub source_span: Option<WireStagingSourceSpan>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4TextBuffer {
    pub text_id: u32,
    pub utf8: String,
    pub mappings: Vec<WireStagingTextMapSegment>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireStagingStyleValue {
    Keyword { value: String },
    String { value: String },
    Integer { value: i64 },
    Length { value: i64 },
    Boolean { value: bool },
    FontFamilyList { families: Vec<String> },
    Ratio { numerator: i64, denominator: u64 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingStyleDeclaration {
    pub name: String,
    pub value: WireStagingStyleValue,
    pub important: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingStyleRule {
    pub style_id: String,
    pub extends: Option<String>,
    pub selector: String,
    pub source_order: u32,
    pub declarations: Vec<WireStagingStyleDeclaration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingStyleSheet {
    pub rules: Vec<WireStagingStyleRule>,
}

/// Typed 1.4 regions plus an opaque carrier for fields whose frozen 1.3 shape
/// is unchanged. Only this module may create the carrier from untrusted JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct WireStagingM4DocumentPackage {
    document: WireStagingM4Document,
    metadata: WireDocumentMetadata,
    outline: WireDocumentOutline,
    resources: WireStagingM4ResourceCatalog,
    sources: Vec<WireStagingM4Source>,
    style_sheet: WireStagingStyleSheet,
    text_buffers: Vec<WireStagingM4TextBuffer>,
    page_masters: WirePageMasterSet,
    advanced_page_masters: WireAdvancedPageMasterSet,
    carrier: Value,
    limits: ValidatedResourceLimits,
}

impl WireStagingM4DocumentPackage {
    pub const fn document(&self) -> &WireStagingM4Document {
        &self.document
    }

    pub const fn resources(&self) -> &WireStagingM4ResourceCatalog {
        &self.resources
    }

    pub const fn metadata(&self) -> &WireDocumentMetadata {
        &self.metadata
    }

    pub const fn outline(&self) -> &WireDocumentOutline {
        &self.outline
    }

    pub fn sources(&self) -> &[WireStagingM4Source] {
        &self.sources
    }

    pub const fn style_sheet(&self) -> &WireStagingStyleSheet {
        &self.style_sheet
    }

    pub fn text_buffers(&self) -> &[WireStagingM4TextBuffer] {
        &self.text_buffers
    }

    pub const fn page_masters(&self) -> &WirePageMasterSet {
        &self.page_masters
    }

    pub const fn advanced_page_masters(&self) -> &WireAdvancedPageMasterSet {
        &self.advanced_page_masters
    }

    pub fn replace_typed_regions(
        &mut self,
        document: WireStagingM4Document,
        resources: WireStagingM4ResourceCatalog,
    ) {
        self.document = document;
        self.resources = resources;
    }

    pub fn replace_style_sheet(&mut self, style_sheet: WireStagingStyleSheet) {
        self.style_sheet = style_sheet;
    }

    pub fn replace_book_navigation(
        &mut self,
        metadata: WireDocumentMetadata,
        outline: WireDocumentOutline,
    ) {
        self.metadata = metadata;
        self.outline = outline;
    }

    fn materialize(&self) -> Result<Value, StagingSemanticDecodeError> {
        let mut value = self.carrier.clone();
        let object = value
            .as_object_mut()
            .ok_or(StagingSemanticDecodeError::Shape(
                "root carrier is not an object",
            ))?;
        object.insert(
            "document".to_owned(),
            serde_json::to_value(&self.document).map_err(StagingSemanticDecodeError::Json)?,
        );
        object.insert(
            "metadata".to_owned(),
            serde_json::to_value(&self.metadata).map_err(StagingSemanticDecodeError::Json)?,
        );
        object.insert(
            "outline".to_owned(),
            serde_json::to_value(&self.outline).map_err(StagingSemanticDecodeError::Json)?,
        );
        object.insert(
            "resources".to_owned(),
            serde_json::to_value(&self.resources).map_err(StagingSemanticDecodeError::Json)?,
        );
        object.insert(
            "sources".to_owned(),
            serde_json::to_value(&self.sources).map_err(StagingSemanticDecodeError::Json)?,
        );
        object.insert(
            "style_sheet".to_owned(),
            serde_json::to_value(&self.style_sheet).map_err(StagingSemanticDecodeError::Json)?,
        );
        object.insert(
            "text_buffers".to_owned(),
            serde_json::to_value(&self.text_buffers).map_err(StagingSemanticDecodeError::Json)?,
        );
        Ok(value)
    }
}

#[derive(Debug)]
pub enum StagingSemanticDecodeError {
    Preflight(JsonPreflightError),
    Json(serde_json::Error),
    Contract,
    Shape(&'static str),
    BookNavigationShape {
        pointer: String,
        message: &'static str,
    },
    PrecomposedVectorShape {
        pointer: String,
        message: &'static str,
    },
    Limit,
}

impl StagingSemanticDecodeError {
    pub fn pointer(&self) -> Option<&str> {
        match self {
            Self::BookNavigationShape { pointer, .. }
            | Self::PrecomposedVectorShape { pointer, .. } => Some(pointer),
            Self::Preflight(_) | Self::Json(_) | Self::Contract | Self::Shape(_) | Self::Limit => {
                None
            }
        }
    }
}

impl fmt::Display for StagingSemanticDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "invalid contract-1.4 JSON: {error}"),
            Self::Contract => formatter.write_str("expected private typaxis.contract/1.4"),
            Self::Shape(message) => write!(formatter, "invalid contract-1.4 shape: {message}"),
            Self::BookNavigationShape { pointer, message } => {
                write!(
                    formatter,
                    "P1102: invalid contract-1.4 shape at {pointer}: {message}"
                )
            }
            Self::PrecomposedVectorShape { pointer, message } => {
                write!(
                    formatter,
                    "P1102: invalid precomposed-vector shape at {pointer}: {message}"
                )
            }
            Self::Limit => formatter.write_str("contract-1.4 package exceeds a resource limit"),
        }
    }
}

impl std::error::Error for StagingSemanticDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Contract
            | Self::Shape(_)
            | Self::BookNavigationShape { .. }
            | Self::PrecomposedVectorShape { .. }
            | Self::Limit => None,
        }
    }
}

pub struct DecodedStagingSemanticDocumentPackage {
    wire: WireStagingM4DocumentPackage,
    limits: ValidatedResourceLimits,
    raw_sha256: [u8; 32],
    canonical_jcs: String,
    canonical_jcs_sha256: [u8; 32],
}

impl fmt::Debug for DecodedStagingSemanticDocumentPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedStagingSemanticDocumentPackage")
            .field("contract", &STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT)
            .field("blocks", &self.wire.document.blocks.len())
            .field("resources", &self.wire.resources)
            .finish_non_exhaustive()
    }
}

impl DecodedStagingSemanticDocumentPackage {
    pub const fn wire(&self) -> &WireStagingM4DocumentPackage {
        &self.wire
    }
    pub const fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn canonical_jcs_sha256(&self) -> [u8; 32] {
        self.canonical_jcs_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn into_wire(self) -> WireStagingM4DocumentPackage {
        self.wire
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingSemanticDocumentPackageDecoder;

impl StagingSemanticDocumentPackageDecoder {
    pub const fn new() -> Self {
        Self
    }

    pub fn decode(
        &self,
        input: &[u8],
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<DecodedStagingSemanticDocumentPackage, StagingSemanticDecodeError> {
        StrictJsonPreflight::new(policy.preflight_limits())
            .check(input)
            .map_err(StagingSemanticDecodeError::Preflight)?;
        let mut deserializer = serde_json::Deserializer::from_slice(input);
        deserializer.disable_recursion_limit();
        let stacker = serde_stacker::Deserializer::new(&mut deserializer);
        let root = NoDuplicateValue::deserialize(stacker)
            .map_err(StagingSemanticDecodeError::Json)?
            .0;
        deserializer
            .end()
            .map_err(StagingSemanticDecodeError::Json)?;

        let object = root
            .as_object()
            .ok_or(StagingSemanticDecodeError::Shape("root must be an object"))?;
        if object.get("contract").and_then(Value::as_str)
            != Some(STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT)
        {
            return Err(StagingSemanticDecodeError::Contract);
        }
        validate_book_navigation_wire_shape(&root)?;
        validate_precomposed_vector_wire_shape(&root)?;
        let expected: BTreeSet<&str> = [
            "contract",
            "coordinate_unit",
            "document",
            "metadata",
            "outline",
            "page_masters",
            "resources",
            "sources",
            "style_sheet",
            "text_buffers",
        ]
        .into_iter()
        .collect();
        if object.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
            return Err(StagingSemanticDecodeError::Shape(
                "root members differ from the contract-1.4 scaffold",
            ));
        }
        if object.get("coordinate_unit").and_then(Value::as_str) != Some("pdf_point_1_65536") {
            return Err(StagingSemanticDecodeError::Shape(
                "coordinate_unit must be pdf_point_1_65536",
            ));
        }
        let (page_masters, advanced_page_masters) = validate_frozen_carrier(&root, policy)?;
        let document: WireStagingM4Document = serde_json::from_value(
            object
                .get("document")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("document is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        validate_semantic_container_shape(&document)?;
        validate_math_wire(&document)?;
        let metadata: WireDocumentMetadata = serde_json::from_value(
            object
                .get("metadata")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("metadata is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        let outline: WireDocumentOutline = serde_json::from_value(
            object
                .get("outline")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("outline is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        let resources: WireStagingM4ResourceCatalog = serde_json::from_value(
            object
                .get("resources")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("resources is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        let sources: Vec<WireStagingM4Source> = serde_json::from_value(
            object
                .get("sources")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("sources is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        let style_sheet: WireStagingStyleSheet = serde_json::from_value(
            object
                .get("style_sheet")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("style_sheet is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        let text_buffers: Vec<WireStagingM4TextBuffer> =
            serde_json::from_value(object.get("text_buffers").cloned().ok_or(
                StagingSemanticDecodeError::Shape("text_buffers is required"),
            )?)
            .map_err(StagingSemanticDecodeError::Json)?;
        validate_supporting_shapes(&sources, &text_buffers)?;
        reject_page_region_semantic_containers(object.get("page_masters").ok_or(
            StagingSemanticDecodeError::Shape("page_masters is required"),
        )?)?;
        if staging_m4_ast_node_count_parts(
            &document,
            &advanced_page_masters,
            metadata.keywords.len(),
            outline.entries.len(),
            policy.resource_limits().get().max_ast_nesting_depth,
        )? > policy.resource_limits().get().max_ast_nodes
            || policy.resource_limits().get().max_ast_nesting_depth < 2
            || outline.entries.iter().any(|entry| {
                u32::from(entry.level).checked_add(2).map_or(true, |depth| {
                    depth > policy.resource_limits().get().max_ast_nesting_depth
                })
            })
            || u64::try_from(resources.font_faces.len())
                .map_err(|_| StagingSemanticDecodeError::Limit)?
                > u64::from(policy.resource_limits().get().max_fonts)
            || u64::try_from(resources.images.len())
                .map_err(|_| StagingSemanticDecodeError::Limit)?
                > u64::from(policy.resource_limits().get().max_images)
            || u64::try_from(style_sheet.rules.len())
                .map_err(|_| StagingSemanticDecodeError::Limit)?
                > policy.resource_limits().get().max_style_rules
        {
            return Err(StagingSemanticDecodeError::Limit);
        }
        let canonical_jcs = canonicalize_value(&root, input.len())?;
        if u64::try_from(canonical_jcs.len()).map_err(|_| StagingSemanticDecodeError::Limit)?
            > policy.resource_limits().get().max_document_package_bytes
        {
            return Err(StagingSemanticDecodeError::Limit);
        }
        Ok(DecodedStagingSemanticDocumentPackage {
            wire: WireStagingM4DocumentPackage {
                document,
                metadata,
                outline,
                resources,
                sources,
                style_sheet,
                text_buffers,
                page_masters,
                advanced_page_masters,
                carrier: root,
                limits: policy.resource_limits().clone(),
            },
            limits: policy.resource_limits().clone(),
            raw_sha256: sha256(input),
            canonical_jcs_sha256: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }
}

/// Validate every unchanged 1.3 carrier field with its existing exact-pinned
/// decoder. Semantic wrappers are removed only in this temporary validation
/// view, and the new required media members are stripped; the original 1.4
/// value remains intact for typed lowering and canonical re-encoding.
fn validate_frozen_carrier(
    root: &Value,
    policy: &DocumentPackageDecodePolicy<'_>,
) -> Result<(WirePageMasterSet, WireAdvancedPageMasterSet), StagingSemanticDecodeError> {
    let mut compatibility = root.clone();
    let object = compatibility
        .as_object_mut()
        .ok_or(StagingSemanticDecodeError::Shape("root must be an object"))?;
    object.insert(
        "contract".to_owned(),
        Value::String("typaxis.contract/1.3".to_owned()),
    );
    object.remove("metadata");
    object.remove("outline");
    let document = object
        .get_mut("document")
        .and_then(Value::as_object_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "document must be an object",
        ))?;
    document.remove("language");
    flatten_semantic_blocks(document.get_mut("blocks").ok_or(
        StagingSemanticDecodeError::Shape("document blocks are required"),
    )?)?;
    if let Some(footnotes) = document.get_mut("footnotes").and_then(Value::as_array_mut) {
        for footnote in footnotes {
            let footnote = footnote
                .as_object_mut()
                .ok_or(StagingSemanticDecodeError::Shape(
                    "footnote must be an object",
                ))?;
            footnote.remove("language");
            let blocks = footnote
                .get_mut("blocks")
                .ok_or(StagingSemanticDecodeError::Shape(
                    "footnote blocks are required",
                ))?;
            flatten_semantic_blocks(blocks)?;
        }
    }
    let style_rules = object
        .get_mut("style_sheet")
        .and_then(Value::as_object_mut)
        .and_then(|sheet| sheet.get_mut("rules"))
        .and_then(Value::as_array_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "style_sheet rules are required",
        ))?;
    for rule in style_rules {
        let selector = rule
            .as_object_mut()
            .and_then(|rule| rule.get_mut("selector"))
            .and_then(|value| value.as_str())
            .ok_or(StagingSemanticDecodeError::Shape(
                "style selector is required",
            ))?
            .to_owned();
        if let Some(classes) = selector.strip_prefix("display_math") {
            rule.as_object_mut()
                .and_then(|rule| rule.get_mut("selector"))
                .ok_or(StagingSemanticDecodeError::Shape(
                    "style selector is required",
                ))?
                .clone_from(&Value::String(format!("paragraph{classes}")));
        }
    }
    let resources = object
        .get_mut("resources")
        .and_then(Value::as_object_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "resources must be an object",
        ))?;
    for key in ["font_faces", "images"] {
        let values = resources.get_mut(key).and_then(Value::as_array_mut).ok_or(
            StagingSemanticDecodeError::Shape("resource catalog arrays are required"),
        )?;
        for value in values {
            value
                .as_object_mut()
                .ok_or(StagingSemanticDecodeError::Shape(
                    "resource declaration must be an object",
                ))?
                .remove("media_type");
            value
                .as_object_mut()
                .expect("resource declaration was checked above")
                .remove("vector_provenance");
        }
    }
    let bytes = canonicalize_value(&compatibility, 0)?;
    let decoded = crate::StagingAdvancedDocumentPackageDecoder::new()
        .decode(bytes.as_bytes(), policy)
        .map_err(|_| {
            StagingSemanticDecodeError::Shape("unchanged contract-1.3 carrier is invalid")
        })?;
    let (wire, advanced_page_masters, _, _, _, _) = decoded.into_parts();
    Ok((wire.page_masters, advanced_page_masters))
}

fn flatten_semantic_blocks(value: &mut Value) -> Result<(), StagingSemanticDecodeError> {
    let blocks = value
        .as_array_mut()
        .ok_or(StagingSemanticDecodeError::Shape(
            "block collection must be an array",
        ))?;
    let mut flattened = Vec::new();
    flattened
        .try_reserve_exact(blocks.len())
        .map_err(|_| StagingSemanticDecodeError::Limit)?;
    for mut block in std::mem::take(blocks) {
        let object = block
            .as_object_mut()
            .ok_or(StagingSemanticDecodeError::Shape("block must be an object"))?;
        object.remove("language");
        match object.get("kind").and_then(Value::as_str) {
            Some("semantic_container") => {
                let mut children =
                    object
                        .remove("blocks")
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "semantic blocks are required",
                        ))?;
                flatten_semantic_blocks(&mut children)?;
                let children = children
                    .as_array_mut()
                    .ok_or(StagingSemanticDecodeError::Shape(
                        "semantic blocks must be an array",
                    ))?;
                flattened.append(children);
            }
            Some("list") => {
                let items = object
                    .get_mut("items")
                    .and_then(Value::as_array_mut)
                    .ok_or(StagingSemanticDecodeError::Shape("list items are required"))?;
                for item in items {
                    let item = item
                        .as_object_mut()
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "list item must be an object",
                        ))?;
                    item.remove("language");
                    let children =
                        item.get_mut("blocks")
                            .ok_or(StagingSemanticDecodeError::Shape(
                                "list-item blocks are required",
                            ))?;
                    flatten_semantic_blocks(children)?;
                }
                flattened.push(block);
            }
            Some("table") => {
                for section in ["head", "body"] {
                    let rows = object
                        .get_mut(section)
                        .and_then(Value::as_array_mut)
                        .ok_or(StagingSemanticDecodeError::Shape("table rows are required"))?;
                    for row in rows {
                        let row = row
                            .as_object_mut()
                            .ok_or(StagingSemanticDecodeError::Shape(
                                "table row must be an object",
                            ))?;
                        row.remove("language");
                        let cells = row.get_mut("cells").and_then(Value::as_array_mut).ok_or(
                            StagingSemanticDecodeError::Shape("table cells are required"),
                        )?;
                        for cell in cells {
                            let cell =
                                cell.as_object_mut()
                                    .ok_or(StagingSemanticDecodeError::Shape(
                                        "table cell must be an object",
                                    ))?;
                            cell.remove("language");
                            let children =
                                cell.get_mut("blocks")
                                    .ok_or(StagingSemanticDecodeError::Shape(
                                        "table-cell blocks are required",
                                    ))?;
                            flatten_semantic_blocks(children)?;
                        }
                    }
                }
                flattened.push(block);
            }
            Some("figure") => {
                let caption =
                    object
                        .get_mut("caption")
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "figure caption is required",
                        ))?;
                flatten_semantic_blocks(caption)?;
                flattened.push(block);
            }
            Some("vector_figure") => {
                let caption =
                    object
                        .get_mut("caption")
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "vector_figure caption is required",
                        ))?;
                flatten_semantic_blocks(caption)?;
                object.insert("kind".to_owned(), Value::String("page_break".to_owned()));
                for member in ["alt", "caption", "image_id", "viewport"] {
                    object.remove(member);
                }
                flattened.push(block);
            }
            Some("paragraph" | "heading") => {
                let children =
                    object
                        .get_mut("children")
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "inline children are required",
                        ))?;
                rewrite_math_inlines(children)?;
                flattened.push(block);
            }
            Some("display_math") => {
                object.insert("kind".to_owned(), Value::String("page_break".to_owned()));
                object.remove("math_source");
                object.remove("speech");
                flattened.push(block);
            }
            Some("math_vector_block") => {
                object.insert("kind".to_owned(), Value::String("page_break".to_owned()));
                for member in [
                    "actual_text",
                    "alt",
                    "equation_number",
                    "image_id",
                    "metrics",
                    "source_tex",
                ] {
                    object.remove(member);
                }
                flattened.push(block);
            }
            _ => flattened.push(block),
        }
    }
    *blocks = flattened;
    Ok(())
}

fn rewrite_math_inlines(value: &mut Value) -> Result<(), StagingSemanticDecodeError> {
    let values = value
        .as_array_mut()
        .ok_or(StagingSemanticDecodeError::Shape(
            "inline collection must be an array",
        ))?;
    for value in values {
        let object = value
            .as_object_mut()
            .ok_or(StagingSemanticDecodeError::Shape(
                "inline must be an object",
            ))?;
        object.remove("language");
        match object.get("kind").and_then(Value::as_str) {
            Some("inline_math") => {
                let text_span = object
                    .remove("math_source")
                    .and_then(|source| source.as_object().cloned())
                    .and_then(|mut source| source.remove("text_span"))
                    .ok_or(StagingSemanticDecodeError::Shape(
                        "math_source text_span is required",
                    ))?;
                object.insert("kind".to_owned(), Value::String("text".to_owned()));
                object.insert("text_span".to_owned(), text_span);
                object.remove("speech");
            }
            Some("inline_vector" | "math_vector") => {
                object.insert("kind".to_owned(), Value::String("soft_break".to_owned()));
                for member in [
                    "actual_text",
                    "alt",
                    "image_id",
                    "metrics",
                    "source_tex",
                    "spacing",
                ] {
                    object.remove(member);
                }
            }
            Some("emphasis" | "strong" | "link") => {
                let children =
                    object
                        .get_mut("children")
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "inline children are required",
                        ))?;
                rewrite_math_inlines(children)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_precomposed_vector_wire_shape(root: &Value) -> Result<(), StagingSemanticDecodeError> {
    fn shape(pointer: impl Into<String>, message: &'static str) -> StagingSemanticDecodeError {
        StagingSemanticDecodeError::PrecomposedVectorShape {
            pointer: pointer.into(),
            message,
        }
    }

    fn object<'a>(
        value: &'a Value,
        pointer: &str,
    ) -> Result<&'a Map<String, Value>, StagingSemanticDecodeError> {
        value
            .as_object()
            .ok_or_else(|| shape(pointer, "expected an object"))
    }

    fn pointer_member(pointer: &str, member: &str) -> String {
        let escaped = member.replace('~', "~0").replace('/', "~1");
        format!("{pointer}/{escaped}")
    }

    fn exact_members(
        value: &Map<String, Value>,
        pointer: &str,
        required: &[&str],
        optional: &[&str],
    ) -> Result<(), StagingSemanticDecodeError> {
        for member in required {
            if !value.contains_key(*member) {
                return Err(shape(
                    pointer_member(pointer, member),
                    "required member is missing",
                ));
            }
        }
        for member in value.keys() {
            if !required.contains(&member.as_str()) && !optional.contains(&member.as_str()) {
                return Err(shape(
                    pointer_member(pointer, member),
                    "member is forbidden for this kind",
                ));
            }
        }
        Ok(())
    }

    fn integer(
        value: Option<&Value>,
        pointer: &str,
        minimum: i64,
    ) -> Result<(), StagingSemanticDecodeError> {
        let value = value
            .and_then(Value::as_i64)
            .filter(|value| {
                *value >= minimum
                    && *value >= -JSON_SAFE_INTEGER_MAX
                    && *value <= JSON_SAFE_INTEGER_MAX
            })
            .ok_or_else(|| shape(pointer, "expected a canonical fixed-point integer in range"))?;
        let _ = value;
        Ok(())
    }

    fn id32(value: Option<&Value>, pointer: &str) -> Result<u64, StagingSemanticDecodeError> {
        value
            .and_then(Value::as_u64)
            .filter(|value| *value <= u64::from(u32::MAX))
            .ok_or_else(|| shape(pointer, "expected an unsigned 32-bit integer"))
    }

    fn required_string<'a>(
        value: Option<&'a Value>,
        pointer: &str,
    ) -> Result<&'a str, StagingSemanticDecodeError> {
        value
            .and_then(Value::as_str)
            .ok_or_else(|| shape(pointer, "expected a string"))
    }

    fn optional_string(
        value: Option<&Value>,
        pointer: &str,
    ) -> Result<(), StagingSemanticDecodeError> {
        if match value {
            None => true,
            Some(value) => value.is_string(),
        } {
            Ok(())
        } else {
            Err(shape(pointer, "expected a string"))
        }
    }

    fn string_array(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let values = value
            .as_array()
            .ok_or_else(|| shape(pointer, "expected an array"))?;
        for (index, value) in values.iter().enumerate() {
            required_string(Some(value), &format!("{pointer}/{index}"))?;
        }
        Ok(())
    }

    fn source_span(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let value = object(value, pointer)?;
        exact_members(
            value,
            pointer,
            &["end_byte", "source_id", "start_byte"],
            &[],
        )?;
        id32(value.get("end_byte"), &format!("{pointer}/end_byte"))?;
        id32(value.get("source_id"), &format!("{pointer}/source_id"))?;
        id32(value.get("start_byte"), &format!("{pointer}/start_byte"))?;
        Ok(())
    }

    fn text_span(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let value = object(value, pointer)?;
        exact_members(value, pointer, &["end_byte", "start_byte", "text_id"], &[])?;
        id32(value.get("end_byte"), &format!("{pointer}/end_byte"))?;
        id32(value.get("start_byte"), &format!("{pointer}/start_byte"))?;
        id32(value.get("text_id"), &format!("{pointer}/text_id"))?;
        Ok(())
    }

    fn viewport(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let value = object(value, pointer)?;
        exact_members(value, pointer, &["height", "width"], &[])?;
        integer(value.get("height"), &format!("{pointer}/height"), 1)?;
        integer(value.get("width"), &format!("{pointer}/width"), 1)
    }

    fn metrics(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let value = object(value, pointer)?;
        exact_members(
            value,
            pointer,
            &[
                "advance", "ascent", "baseline", "descent", "origin_x", "viewport",
            ],
            &[],
        )?;
        integer(value.get("advance"), &format!("{pointer}/advance"), 1)?;
        integer(value.get("ascent"), &format!("{pointer}/ascent"), 1)?;
        integer(value.get("baseline"), &format!("{pointer}/baseline"), 0)?;
        integer(value.get("descent"), &format!("{pointer}/descent"), 0)?;
        integer(
            value.get("origin_x"),
            &format!("{pointer}/origin_x"),
            -JSON_SAFE_INTEGER_MAX,
        )?;
        viewport(
            value.get("viewport").ok_or_else(|| {
                shape(format!("{pointer}/viewport"), "required member is missing")
            })?,
            &format!("{pointer}/viewport"),
        )
    }

    fn spacing(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let value = object(value, pointer)?;
        exact_members(value, pointer, &["after", "before"], &[])?;
        integer(value.get("after"), &format!("{pointer}/after"), 0)?;
        integer(value.get("before"), &format!("{pointer}/before"), 0)
    }

    fn source_tex(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let value = object(value, pointer)?;
        exact_members(value, pointer, &["text_span"], &[])?;
        text_span(
            value.get("text_span").ok_or_else(|| {
                shape(format!("{pointer}/text_span"), "required member is missing")
            })?,
            &format!("{pointer}/text_span"),
        )
    }

    fn nullable_text(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        if value.is_null() || value.is_string() {
            Ok(())
        } else {
            Err(shape(pointer, "expected string or null"))
        }
    }

    fn equation_number(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        if value.is_null() {
            return Ok(());
        }
        let value = object(value, pointer)?;
        exact_members(
            value,
            pointer,
            &["minimum_gap", "node_id", "span", "text_span"],
            &[],
        )?;
        integer(
            value.get("minimum_gap"),
            &format!("{pointer}/minimum_gap"),
            1,
        )?;
        id32(value.get("node_id"), &format!("{pointer}/node_id"))?;
        source_span(
            value
                .get("span")
                .ok_or_else(|| shape(format!("{pointer}/span"), "required member is missing"))?,
            &format!("{pointer}/span"),
        )?;
        text_span(
            value.get("text_span").ok_or_else(|| {
                shape(format!("{pointer}/text_span"), "required member is missing")
            })?,
            &format!("{pointer}/text_span"),
        )
    }

    fn validate_inline_vector(
        value: &Map<String, Value>,
        pointer: &str,
        math: bool,
    ) -> Result<(), StagingSemanticDecodeError> {
        let mut required = vec![
            "actual_text",
            "alt",
            "image_id",
            "kind",
            "metrics",
            "node_id",
            "spacing",
            "span",
        ];
        if math {
            required.push("source_tex");
        }
        exact_members(value, pointer, &required, &["language"])?;
        nullable_text(
            value.get("actual_text").ok_or_else(|| {
                shape(
                    format!("{pointer}/actual_text"),
                    "required member is missing",
                )
            })?,
            &format!("{pointer}/actual_text"),
        )?;
        if !value.get("alt").is_some_and(Value::is_string) {
            return Err(shape(format!("{pointer}/alt"), "expected a string"));
        }
        id32(value.get("image_id"), &format!("{pointer}/image_id"))?;
        id32(value.get("node_id"), &format!("{pointer}/node_id"))?;
        source_span(
            value
                .get("span")
                .ok_or_else(|| shape(format!("{pointer}/span"), "required member is missing"))?,
            &format!("{pointer}/span"),
        )?;
        optional_string(value.get("language"), &format!("{pointer}/language"))?;
        metrics(
            value
                .get("metrics")
                .ok_or_else(|| shape(format!("{pointer}/metrics"), "required member is missing"))?,
            &format!("{pointer}/metrics"),
        )?;
        spacing(
            value
                .get("spacing")
                .ok_or_else(|| shape(format!("{pointer}/spacing"), "required member is missing"))?,
            &format!("{pointer}/spacing"),
        )?;
        if math {
            source_tex(
                value.get("source_tex").ok_or_else(|| {
                    shape(
                        format!("{pointer}/source_tex"),
                        "required member is missing",
                    )
                })?,
                &format!("{pointer}/source_tex"),
            )?;
        }
        Ok(())
    }

    fn visit_inlines(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let values = value
            .as_array()
            .ok_or_else(|| shape(pointer, "expected an inline array"))?;
        for (index, value) in values.iter().enumerate() {
            let pointer = format!("{pointer}/{index}");
            let object = object(value, &pointer)?;
            let kind_pointer = format!("{pointer}/kind");
            let kind = required_string(object.get("kind"), &kind_pointer)?;
            match kind {
                "inline_vector" => validate_inline_vector(object, &pointer, false)?,
                "math_vector" => validate_inline_vector(object, &pointer, true)?,
                "emphasis" | "strong" | "link" => {
                    if let Some(children) = object.get("children") {
                        visit_inlines(children, &format!("{pointer}/children"))?;
                    }
                }
                "text" | "inline_math" | "anchor" | "reference" | "footnote_reference"
                | "soft_break" | "hard_break" => {}
                _ => return Err(shape(kind_pointer, "unsupported inline kind")),
            }
        }
        Ok(())
    }

    fn visit_blocks(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        let values = value
            .as_array()
            .ok_or_else(|| shape(pointer, "expected a block array"))?;
        for (index, value) in values.iter().enumerate() {
            let pointer = format!("{pointer}/{index}");
            let object = object(value, &pointer)?;
            let kind_pointer = format!("{pointer}/kind");
            let kind = required_string(object.get("kind"), &kind_pointer)?;
            match kind {
                "paragraph" | "heading" => {
                    if let Some(children) = object.get("children") {
                        visit_inlines(children, &format!("{pointer}/children"))?;
                    }
                }
                "list" => {
                    if let Some(items) = object.get("items").and_then(Value::as_array) {
                        for (item_index, item) in items.iter().enumerate() {
                            if let Some(blocks) =
                                item.as_object().and_then(|item| item.get("blocks"))
                            {
                                visit_blocks(
                                    blocks,
                                    &format!("{pointer}/items/{item_index}/blocks"),
                                )?;
                            }
                        }
                    }
                }
                "table" => {
                    for section in ["head", "body"] {
                        if let Some(rows) = object.get(section).and_then(Value::as_array) {
                            for (row_index, row) in rows.iter().enumerate() {
                                if let Some(cells) = row
                                    .as_object()
                                    .and_then(|row| row.get("cells"))
                                    .and_then(Value::as_array)
                                {
                                    for (cell_index, cell) in cells.iter().enumerate() {
                                        if let Some(blocks) =
                                            cell.as_object().and_then(|cell| cell.get("blocks"))
                                        {
                                            visit_blocks(
                                                blocks,
                                                &format!(
                                                    "{pointer}/{section}/{row_index}/cells/{cell_index}/blocks"
                                                ),
                                            )?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "figure" => {
                    if let Some(caption) = object.get("caption") {
                        visit_blocks(caption, &format!("{pointer}/caption"))?;
                    }
                }
                "semantic_container" => {
                    if let Some(blocks) = object.get("blocks") {
                        visit_blocks(blocks, &format!("{pointer}/blocks"))?;
                    }
                }
                "vector_figure" => {
                    exact_members(
                        object,
                        &pointer,
                        &[
                            "alt", "caption", "classes", "image_id", "kind", "node_id", "span",
                            "viewport",
                        ],
                        &["language"],
                    )?;
                    if !object.get("alt").is_some_and(Value::is_string) {
                        return Err(shape(format!("{pointer}/alt"), "expected a string"));
                    }
                    string_array(
                        object.get("classes").ok_or_else(|| {
                            shape(format!("{pointer}/classes"), "required member is missing")
                        })?,
                        &format!("{pointer}/classes"),
                    )?;
                    id32(object.get("image_id"), &format!("{pointer}/image_id"))?;
                    id32(object.get("node_id"), &format!("{pointer}/node_id"))?;
                    source_span(
                        object.get("span").ok_or_else(|| {
                            shape(format!("{pointer}/span"), "required member is missing")
                        })?,
                        &format!("{pointer}/span"),
                    )?;
                    optional_string(object.get("language"), &format!("{pointer}/language"))?;
                    viewport(
                        object.get("viewport").ok_or_else(|| {
                            shape(format!("{pointer}/viewport"), "required member is missing")
                        })?,
                        &format!("{pointer}/viewport"),
                    )?;
                    visit_blocks(
                        object.get("caption").ok_or_else(|| {
                            shape(format!("{pointer}/caption"), "required member is missing")
                        })?,
                        &format!("{pointer}/caption"),
                    )?;
                }
                "math_vector_block" => {
                    exact_members(
                        object,
                        &pointer,
                        &[
                            "actual_text",
                            "alt",
                            "classes",
                            "equation_number",
                            "image_id",
                            "kind",
                            "metrics",
                            "node_id",
                            "source_tex",
                            "span",
                        ],
                        &["language"],
                    )?;
                    nullable_text(
                        object.get("actual_text").ok_or_else(|| {
                            shape(
                                format!("{pointer}/actual_text"),
                                "required member is missing",
                            )
                        })?,
                        &format!("{pointer}/actual_text"),
                    )?;
                    if !object.get("alt").is_some_and(Value::is_string) {
                        return Err(shape(format!("{pointer}/alt"), "expected a string"));
                    }
                    string_array(
                        object.get("classes").ok_or_else(|| {
                            shape(format!("{pointer}/classes"), "required member is missing")
                        })?,
                        &format!("{pointer}/classes"),
                    )?;
                    id32(object.get("image_id"), &format!("{pointer}/image_id"))?;
                    id32(object.get("node_id"), &format!("{pointer}/node_id"))?;
                    source_span(
                        object.get("span").ok_or_else(|| {
                            shape(format!("{pointer}/span"), "required member is missing")
                        })?,
                        &format!("{pointer}/span"),
                    )?;
                    optional_string(object.get("language"), &format!("{pointer}/language"))?;
                    metrics(
                        object.get("metrics").ok_or_else(|| {
                            shape(format!("{pointer}/metrics"), "required member is missing")
                        })?,
                        &format!("{pointer}/metrics"),
                    )?;
                    source_tex(
                        object.get("source_tex").ok_or_else(|| {
                            shape(
                                format!("{pointer}/source_tex"),
                                "required member is missing",
                            )
                        })?,
                        &format!("{pointer}/source_tex"),
                    )?;
                    equation_number(
                        object.get("equation_number").ok_or_else(|| {
                            shape(
                                format!("{pointer}/equation_number"),
                                "required member is missing",
                            )
                        })?,
                        &format!("{pointer}/equation_number"),
                    )?;
                }
                "page_break" | "display_math" => {}
                _ => return Err(shape(kind_pointer, "unsupported block kind")),
            }
        }
        Ok(())
    }

    let root_object = root
        .as_object()
        .ok_or(StagingSemanticDecodeError::Shape("root must be an object"))?;
    if let Some(images) = root_object
        .get("resources")
        .and_then(Value::as_object)
        .and_then(|resources| resources.get("images"))
        .and_then(Value::as_array)
    {
        let mut image_ids = BTreeSet::new();
        for (index, image) in images.iter().enumerate() {
            let pointer = format!("/resources/images/{index}");
            let image = object(image, &pointer)?;
            let image_id = id32(image.get("image_id"), &format!("{pointer}/image_id"))?;
            if !image_ids.insert(image_id) {
                return Err(shape(
                    format!("{pointer}/image_id"),
                    "duplicate image_id declaration",
                ));
            }
            if image.get("media_type").and_then(Value::as_str) == Some("svg-safe-2") {
                exact_members(
                    image,
                    &pointer,
                    &[
                        "expected_sha256",
                        "image_id",
                        "media_type",
                        "uri",
                        "vector_provenance",
                    ],
                    &[],
                )?;
                required_string(image.get("uri"), &format!("{pointer}/uri"))?;
                let hash_pointer = format!("{pointer}/expected_sha256");
                let hash = image
                    .get("expected_sha256")
                    .and_then(Value::as_str)
                    .ok_or_else(|| shape(&hash_pointer, "svg-safe-2 requires a nonnull hash"))?;
                if hash.len() != 64
                    || !hash
                        .as_bytes()
                        .iter()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
                {
                    return Err(shape(hash_pointer, "expected lowercase 64-hex SHA-256"));
                }
                let provenance_pointer = format!("{pointer}/vector_provenance");
                let provenance = object(
                    image.get("vector_provenance").ok_or_else(|| {
                        shape(&provenance_pointer, "svg-safe-2 requires provenance")
                    })?,
                    &provenance_pointer,
                )?;
                exact_members(
                    provenance,
                    &provenance_pointer,
                    &["engine_id", "engine_version", "rules_version"],
                    &[],
                )?;
                for member in ["engine_id", "engine_version", "rules_version"] {
                    let member_pointer = format!("{provenance_pointer}/{member}");
                    let value = required_string(provenance.get(member), &member_pointer)?;
                    if value.is_empty()
                        || value.len() > 128
                        || !value
                            .as_bytes()
                            .iter()
                            .all(|byte| (0x20..=0x7e).contains(byte))
                    {
                        return Err(shape(
                            member_pointer,
                            "expected 1..128 bytes of printable ASCII",
                        ));
                    }
                }
            } else if image.contains_key("vector_provenance") {
                return Err(shape(
                    format!("{pointer}/vector_provenance"),
                    "provenance is forbidden for this media type",
                ));
            }
        }
    }

    if let Some(document) = root_object.get("document").and_then(Value::as_object) {
        if let Some(blocks) = document.get("blocks") {
            visit_blocks(blocks, "/document/blocks")?;
        }
        if let Some(footnotes) = document.get("footnotes").and_then(Value::as_array) {
            for (index, footnote) in footnotes.iter().enumerate() {
                if let Some(blocks) = footnote
                    .as_object()
                    .and_then(|footnote| footnote.get("blocks"))
                {
                    visit_blocks(blocks, &format!("/document/footnotes/{index}/blocks"))?;
                }
            }
        }
    }
    Ok(())
}

fn validate_semantic_container_shape(
    document: &WireStagingM4Document,
) -> Result<(), StagingSemanticDecodeError> {
    fn visit(blocks: &[WireStagingM4Block]) -> Result<(), StagingSemanticDecodeError> {
        for block in blocks {
            match block {
                WireStagingM4Block::SemanticContainer { blocks, .. } => {
                    if blocks.is_empty() {
                        return Err(StagingSemanticDecodeError::Shape(
                            "semantic_container blocks must not be empty",
                        ));
                    }
                    visit(blocks)?;
                }
                WireStagingM4Block::List { items, .. } => {
                    for item in items {
                        visit(&item.blocks)?;
                    }
                }
                WireStagingM4Block::Table { head, body, .. } => {
                    for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                        visit(&cell.blocks)?;
                    }
                }
                WireStagingM4Block::Figure { caption, .. }
                | WireStagingM4Block::VectorFigure { caption, .. } => visit(caption)?,
                WireStagingM4Block::Paragraph { .. }
                | WireStagingM4Block::Heading { .. }
                | WireStagingM4Block::DisplayMath { .. }
                | WireStagingM4Block::MathVectorBlock { .. }
                | WireStagingM4Block::PageBreak { .. } => {}
            }
        }
        Ok(())
    }

    visit(&document.blocks)?;
    for footnote in &document.footnotes {
        visit(&footnote.blocks)?;
    }
    Ok(())
}

fn validate_book_navigation_wire_shape(root: &Value) -> Result<(), StagingSemanticDecodeError> {
    fn shape(pointer: impl Into<String>, message: &'static str) -> StagingSemanticDecodeError {
        StagingSemanticDecodeError::BookNavigationShape {
            pointer: pointer.into(),
            message,
        }
    }

    fn pointer_member(pointer: &str, member: &str) -> String {
        let escaped = member.replace('~', "~0").replace('/', "~1");
        if pointer.is_empty() {
            format!("/{escaped}")
        } else {
            format!("{pointer}/{escaped}")
        }
    }

    fn exact_members(
        value: &Value,
        expected: &[&str],
        pointer: &str,
        message: &'static str,
    ) -> Result<(), StagingSemanticDecodeError> {
        let object = value.as_object().ok_or_else(|| shape(pointer, message))?;
        let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected = expected.iter().copied().collect::<BTreeSet<_>>();
        if actual != expected {
            let responsible = expected
                .difference(&actual)
                .next()
                .copied()
                .or_else(|| actual.difference(&expected).next().copied())
                .map_or_else(
                    || pointer.to_owned(),
                    |member| pointer_member(pointer, member),
                );
            return Err(shape(responsible, message));
        }
        Ok(())
    }

    fn unsigned(value: Option<&Value>, maximum: u64) -> bool {
        value
            .and_then(Value::as_u64)
            .is_some_and(|number| number <= maximum)
    }

    fn visit_languages(value: &Value, pointer: &str) -> Result<(), StagingSemanticDecodeError> {
        match value {
            Value::Object(object) => {
                if let Some(language) = object.get("language") {
                    if !language.is_string() {
                        return Err(shape(
                            pointer_member(pointer, "language"),
                            "node language must be a string",
                        ));
                    }
                    if matches!(
                        object.get("kind").and_then(Value::as_str),
                        Some("anchor" | "soft_break" | "hard_break" | "page_break")
                    ) {
                        return Err(shape(
                            pointer_member(pointer, "language"),
                            "node kind does not admit language",
                        ));
                    }
                }
                if object.get("kind").and_then(Value::as_str) == Some("semantic_container") {
                    let anchor_pointer = pointer_member(pointer, "anchor_id");
                    match object.get("anchor_id") {
                        Some(Value::String(_) | Value::Null) => {}
                        Some(_) => {
                            return Err(shape(
                                anchor_pointer,
                                "semantic_container anchor_id must be a string or null",
                            ));
                        }
                        None => {
                            return Err(shape(
                                anchor_pointer,
                                "semantic_container anchor_id is required",
                            ));
                        }
                    }
                }
                for (member, child) in object {
                    visit_languages(child, &pointer_member(pointer, member))?;
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    visit_languages(child, &format!("{pointer}/{index}"))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    let object = root
        .as_object()
        .ok_or(StagingSemanticDecodeError::Shape("root must be an object"))?;
    let metadata = object
        .get("metadata")
        .ok_or_else(|| shape("/metadata", "metadata is required"))?;
    exact_members(
        metadata,
        &[
            "author",
            "created",
            "identifier",
            "keywords",
            "modified",
            "subject",
            "title",
        ],
        "/metadata",
        "metadata must contain exactly the seven contract members",
    )?;
    let metadata = metadata
        .as_object()
        .ok_or_else(|| shape("/metadata", "metadata must be an object"))?;
    for member in [
        "author",
        "created",
        "identifier",
        "modified",
        "subject",
        "title",
    ] {
        if !matches!(metadata.get(member), Some(Value::String(_) | Value::Null)) {
            return Err(shape(
                pointer_member("/metadata", member),
                "nullable metadata member must be a string or null",
            ));
        }
    }
    let keywords = metadata
        .get("keywords")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            shape(
                "/metadata/keywords",
                "metadata keywords must be an array of strings",
            )
        })?;
    for (index, keyword) in keywords.iter().enumerate() {
        if !keyword.is_string() {
            return Err(shape(
                format!("/metadata/keywords/{index}"),
                "metadata keyword must be a string",
            ));
        }
    }

    let outline = object
        .get("outline")
        .ok_or_else(|| shape("/outline", "outline is required"))?;
    exact_members(
        outline,
        &["entries"],
        "/outline",
        "outline must contain exactly entries",
    )?;
    let entries = outline
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| shape("/outline/entries", "outline entries must be an array"))?;
    for (index, entry) in entries.iter().enumerate() {
        let pointer = format!("/outline/entries/{index}");
        exact_members(
            entry,
            &[
                "destination",
                "label",
                "level",
                "outline_id",
                "parent_outline_id",
                "source_kind",
                "source_node_id",
            ],
            &pointer,
            "outline entry members differ from the contract",
        )?;
        let entry = entry
            .as_object()
            .ok_or_else(|| shape(&pointer, "outline entry must be an object"))?;
        for member in ["destination", "label"] {
            if !entry.get(member).is_some_and(Value::is_string) {
                return Err(shape(
                    pointer_member(&pointer, member),
                    "outline text member must be a string",
                ));
            }
        }
        if !unsigned(entry.get("level"), u64::from(u8::MAX)) {
            return Err(shape(
                pointer_member(&pointer, "level"),
                "outline level must be an unsigned byte",
            ));
        }
        if !unsigned(entry.get("outline_id"), u64::from(u32::MAX)) {
            return Err(shape(
                pointer_member(&pointer, "outline_id"),
                "outline ID must be an id32",
            ));
        }
        if !matches!(entry.get("parent_outline_id"), Some(Value::Null))
            && !unsigned(entry.get("parent_outline_id"), u64::from(u32::MAX))
        {
            return Err(shape(
                pointer_member(&pointer, "parent_outline_id"),
                "outline parent must be an id32 or null",
            ));
        }
        if !matches!(
            entry.get("source_kind").and_then(Value::as_str),
            Some("heading" | "semantic_container")
        ) {
            return Err(shape(
                pointer_member(&pointer, "source_kind"),
                "outline source_kind is unsupported",
            ));
        }
        if !unsigned(entry.get("source_node_id"), u64::from(u32::MAX)) {
            return Err(shape(
                pointer_member(&pointer, "source_node_id"),
                "outline source node ID must be an id32",
            ));
        }
    }

    let document_value = object
        .get("document")
        .ok_or_else(|| shape("/document", "document must be an object"))?;
    let document = document_value
        .as_object()
        .ok_or_else(|| shape("/document", "document must be an object"))?;
    if !document.get("language").is_some_and(Value::is_string) {
        return Err(shape(
            "/document/language",
            "document language must be a string",
        ));
    }
    visit_languages(document_value, "/document")
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingSemanticDocumentPackageEncoder;

impl StagingSemanticDocumentPackageEncoder {
    pub const fn new() -> Self {
        Self
    }

    pub fn encode(
        &self,
        package: &WireStagingM4DocumentPackage,
    ) -> Result<String, StagingSemanticDecodeError> {
        let limits = package.limits.get();
        let node_count = staging_m4_wire_ast_node_count(package, limits.max_ast_nesting_depth)?;
        if node_count > limits.max_ast_nodes
            || limits.max_ast_nesting_depth < 2
            || package.outline.entries.iter().any(|entry| {
                u32::from(entry.level)
                    .checked_add(2)
                    .map_or(true, |depth| depth > limits.max_ast_nesting_depth)
            })
            || u64::try_from(package.resources.font_faces.len())
                .map_err(|_| StagingSemanticDecodeError::Limit)?
                > u64::from(limits.max_fonts)
            || u64::try_from(package.resources.images.len())
                .map_err(|_| StagingSemanticDecodeError::Limit)?
                > u64::from(limits.max_images)
            || u64::try_from(package.style_sheet.rules.len())
                .map_err(|_| StagingSemanticDecodeError::Limit)?
                > limits.max_style_rules
        {
            return Err(StagingSemanticDecodeError::Limit);
        }
        validate_semantic_container_shape(&package.document)?;
        validate_math_wire(&package.document)?;
        validate_book_navigation_wire_shape(&package.materialize()?)?;
        validate_precomposed_vector_wire_shape(&package.materialize()?)?;
        validate_supporting_shapes(&package.sources, &package.text_buffers)?;
        reject_page_region_semantic_containers(&package.carrier["page_masters"])?;
        let canonical = canonicalize_value(&package.materialize()?, 0)?;
        if u64::try_from(canonical.len()).map_err(|_| StagingSemanticDecodeError::Limit)?
            > limits.max_document_package_bytes
        {
            return Err(StagingSemanticDecodeError::Limit);
        }
        Ok(canonical)
    }
}

fn validate_supporting_shapes(
    sources: &[WireStagingM4Source],
    text_buffers: &[WireStagingM4TextBuffer],
) -> Result<(), StagingSemanticDecodeError> {
    for source in sources {
        if source.sha256.len() != 64
            || !source
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(StagingSemanticDecodeError::Shape(
                "source sha256 must be lowercase hexadecimal",
            ));
        }
    }
    for buffer in text_buffers {
        for mapping in &buffer.mappings {
            let requires_source = matches!(
                mapping.kind,
                WireStagingTextMapKind::Identity | WireStagingTextMapKind::Replacement
            );
            if requires_source != mapping.source_span.is_some() {
                return Err(StagingSemanticDecodeError::Shape(
                    "text mapping source_span disagrees with its kind",
                ));
            }
        }
    }
    Ok(())
}

fn reject_page_region_semantic_containers(value: &Value) -> Result<(), StagingSemanticDecodeError> {
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(object) => {
                if matches!(
                    object.get("kind").and_then(Value::as_str),
                    Some(
                        "semantic_container"
                            | "display_math"
                            | "inline_math"
                            | "inline_vector"
                            | "math_vector"
                            | "vector_figure"
                            | "math_vector_block"
                    )
                ) {
                    return Err(StagingSemanticDecodeError::Shape(
                        "semantic_container or math cannot occur in a page region",
                    ));
                }
                stack.extend(object.values());
            }
            Value::Array(values) => stack.extend(values),
            _ => {}
        }
    }
    Ok(())
}

fn validate_math_wire(document: &WireStagingM4Document) -> Result<(), StagingSemanticDecodeError> {
    fn source(value: &WireStagingMathSource) -> Result<(), StagingSemanticDecodeError> {
        if value.language != "typaxis-math" || value.version != "1" {
            return Err(StagingSemanticDecodeError::Shape(
                "math_source language/version is unsupported",
            ));
        }
        Ok(())
    }

    fn inlines(values: &[WireStagingM4Inline]) -> Result<(), StagingSemanticDecodeError> {
        for value in values {
            match value {
                WireStagingM4Inline::InlineMath { math_source, .. } => source(math_source)?,
                WireStagingM4Inline::Emphasis { children, .. }
                | WireStagingM4Inline::Strong { children, .. }
                | WireStagingM4Inline::Link { children, .. } => inlines(children)?,
                WireStagingM4Inline::Text { .. }
                | WireStagingM4Inline::InlineVector { .. }
                | WireStagingM4Inline::MathVector { .. }
                | WireStagingM4Inline::Anchor { .. }
                | WireStagingM4Inline::Reference { .. }
                | WireStagingM4Inline::FootnoteReference { .. }
                | WireStagingM4Inline::SoftBreak { .. }
                | WireStagingM4Inline::HardBreak { .. } => {}
            }
        }
        Ok(())
    }

    fn blocks(values: &[WireStagingM4Block]) -> Result<(), StagingSemanticDecodeError> {
        for value in values {
            match value {
                WireStagingM4Block::Paragraph { children, .. }
                | WireStagingM4Block::Heading { children, .. } => inlines(children)?,
                WireStagingM4Block::List { items, .. } => {
                    for item in items {
                        blocks(&item.blocks)?;
                    }
                }
                WireStagingM4Block::Table { head, body, .. } => {
                    for row in head.iter().chain(body) {
                        for cell in &row.cells {
                            blocks(&cell.blocks)?;
                        }
                    }
                }
                WireStagingM4Block::Figure { caption, .. }
                | WireStagingM4Block::VectorFigure { caption, .. }
                | WireStagingM4Block::SemanticContainer {
                    blocks: caption, ..
                } => blocks(caption)?,
                WireStagingM4Block::DisplayMath { math_source, .. } => source(math_source)?,
                WireStagingM4Block::PageBreak { .. }
                | WireStagingM4Block::MathVectorBlock { .. } => {}
            }
        }
        Ok(())
    }

    blocks(&document.blocks)?;
    for footnote in &document.footnotes {
        blocks(&footnote.blocks)?;
    }
    Ok(())
}

fn document_node_count(
    document: &WireStagingM4Document,
    max_depth: u32,
) -> Result<u64, StagingSemanticDecodeError> {
    fn blocks(
        values: &[WireStagingM4Block],
        count: &mut u64,
        depth: u32,
        max_depth: u32,
    ) -> Result<(), StagingSemanticDecodeError> {
        for block in values {
            if depth > max_depth {
                return Err(StagingSemanticDecodeError::Limit);
            }
            *count = count
                .checked_add(1)
                .ok_or(StagingSemanticDecodeError::Limit)?;
            match block {
                WireStagingM4Block::Paragraph { children, .. }
                | WireStagingM4Block::Heading { children, .. } => {
                    *count = count
                        .checked_add(count_inline_nodes(
                            children,
                            depth
                                .checked_add(1)
                                .ok_or(StagingSemanticDecodeError::Limit)?,
                            max_depth,
                        )?)
                        .ok_or(StagingSemanticDecodeError::Limit)?;
                }
                WireStagingM4Block::List { items, .. } => {
                    for item in items {
                        let item_depth = depth
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?;
                        if item_depth > max_depth {
                            return Err(StagingSemanticDecodeError::Limit);
                        }
                        *count = count
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?;
                        blocks(
                            &item.blocks,
                            count,
                            item_depth
                                .checked_add(1)
                                .ok_or(StagingSemanticDecodeError::Limit)?,
                            max_depth,
                        )?;
                    }
                }
                WireStagingM4Block::Table {
                    columns,
                    head,
                    body,
                    ..
                } => {
                    *count = count
                        .checked_add(
                            u64::try_from(columns.len())
                                .map_err(|_| StagingSemanticDecodeError::Limit)?,
                        )
                        .ok_or(StagingSemanticDecodeError::Limit)?;
                    for row in head.iter().chain(body) {
                        let row_depth = depth
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?;
                        if row_depth > max_depth {
                            return Err(StagingSemanticDecodeError::Limit);
                        }
                        *count = count
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?;
                        for cell in &row.cells {
                            let cell_depth = row_depth
                                .checked_add(1)
                                .ok_or(StagingSemanticDecodeError::Limit)?;
                            if cell_depth > max_depth {
                                return Err(StagingSemanticDecodeError::Limit);
                            }
                            *count = count
                                .checked_add(1)
                                .ok_or(StagingSemanticDecodeError::Limit)?;
                            blocks(
                                &cell.blocks,
                                count,
                                cell_depth
                                    .checked_add(1)
                                    .ok_or(StagingSemanticDecodeError::Limit)?,
                                max_depth,
                            )?;
                        }
                    }
                }
                WireStagingM4Block::Figure { caption, .. }
                | WireStagingM4Block::VectorFigure { caption, .. }
                | WireStagingM4Block::SemanticContainer {
                    blocks: caption, ..
                } => {
                    blocks(
                        caption,
                        count,
                        depth
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?,
                        max_depth,
                    )?;
                }
                WireStagingM4Block::MathVectorBlock {
                    equation_number, ..
                } => {
                    if equation_number.is_some() {
                        let child_depth = depth
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?;
                        if child_depth > max_depth {
                            return Err(StagingSemanticDecodeError::Limit);
                        }
                        *count = count
                            .checked_add(1)
                            .ok_or(StagingSemanticDecodeError::Limit)?;
                    }
                }
                WireStagingM4Block::PageBreak { .. } | WireStagingM4Block::DisplayMath { .. } => {}
            }
        }
        Ok(())
    }

    if max_depth < 1 {
        return Err(StagingSemanticDecodeError::Limit);
    }
    let mut count = 1;
    blocks(&document.blocks, &mut count, 2, max_depth)?;
    for footnote in &document.footnotes {
        if max_depth < 2 {
            return Err(StagingSemanticDecodeError::Limit);
        }
        count = count
            .checked_add(1)
            .ok_or(StagingSemanticDecodeError::Limit)?;
        blocks(&footnote.blocks, &mut count, 3, max_depth)?;
    }
    Ok(count)
}

fn advanced_page_node_count(
    page_masters: &WireAdvancedPageMasterSet,
) -> Result<u64, StagingSemanticDecodeError> {
    fn region(value: &crate::WirePageRegion) -> Result<u64, StagingSemanticDecodeError> {
        let mut count = 1u64;
        for block in &value.blocks {
            count = count
                .checked_add(1)
                .ok_or(StagingSemanticDecodeError::Limit)?;
            let children = match block {
                crate::WirePageRegionBlock::Paragraph { children, .. }
                | crate::WirePageRegionBlock::Heading { children, .. } => children,
            };
            count = count
                .checked_add(
                    u64::try_from(children.len()).map_err(|_| StagingSemanticDecodeError::Limit)?,
                )
                .ok_or(StagingSemanticDecodeError::Limit)?;
        }
        Ok(count)
    }

    let mut count = 0u64;
    for master in &page_masters.masters {
        for value in [
            master.header_content.as_ref(),
            master.footer_content.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            count = count
                .checked_add(region(value)?)
                .ok_or(StagingSemanticDecodeError::Limit)?;
        }
        if master.column_layout.is_some() {
            count = count
                .checked_add(1)
                .ok_or(StagingSemanticDecodeError::Limit)?;
        }
    }
    Ok(count)
}

/// Returns the complete contract-1.4 wire AST charge before parsed math AST
/// nodes are added by the syntax owner.
#[doc(hidden)]
pub fn staging_m4_wire_ast_node_count(
    package: &WireStagingM4DocumentPackage,
    max_depth: u32,
) -> Result<u64, StagingSemanticDecodeError> {
    staging_m4_ast_node_count_parts(
        &package.document,
        &package.advanced_page_masters,
        package.metadata.keywords.len(),
        package.outline.entries.len(),
        max_depth,
    )
}

fn staging_m4_ast_node_count_parts(
    document: &WireStagingM4Document,
    page_masters: &WireAdvancedPageMasterSet,
    keyword_count: usize,
    outline_count: usize,
    max_depth: u32,
) -> Result<u64, StagingSemanticDecodeError> {
    let navigation_nodes = 2u64
        .checked_add(u64::try_from(keyword_count).map_err(|_| StagingSemanticDecodeError::Limit)?)
        .and_then(|count| count.checked_add(u64::try_from(outline_count).ok()?))
        .ok_or(StagingSemanticDecodeError::Limit)?;
    document_node_count(document, max_depth)?
        .checked_add(advanced_page_node_count(page_masters)?)
        .and_then(|count| count.checked_add(navigation_nodes))
        .ok_or(StagingSemanticDecodeError::Limit)
}

fn count_inline_nodes(
    values: &[WireStagingM4Inline],
    depth: u32,
    max_depth: u32,
) -> Result<u64, StagingSemanticDecodeError> {
    let mut count = 0u64;
    let mut stack: Vec<(&WireStagingM4Inline, u32)> =
        values.iter().rev().map(|value| (value, depth)).collect();
    while let Some((value, depth)) = stack.pop() {
        if depth > max_depth {
            return Err(StagingSemanticDecodeError::Limit);
        }
        count = count
            .checked_add(1)
            .ok_or(StagingSemanticDecodeError::Limit)?;
        match value {
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } => {
                let child_depth = depth
                    .checked_add(1)
                    .ok_or(StagingSemanticDecodeError::Limit)?;
                stack.extend(children.iter().rev().map(|child| (child, child_depth)));
            }
            WireStagingM4Inline::InlineMath { .. }
            | WireStagingM4Inline::InlineVector { .. }
            | WireStagingM4Inline::MathVector { .. } => {}
            WireStagingM4Inline::Text { .. }
            | WireStagingM4Inline::Anchor { .. }
            | WireStagingM4Inline::Reference { .. }
            | WireStagingM4Inline::FootnoteReference { .. }
            | WireStagingM4Inline::SoftBreak { .. }
            | WireStagingM4Inline::HardBreak { .. } => {}
        }
    }
    Ok(count)
}

struct NoDuplicateValue(Value);

impl<'de> Deserialize<'de> for NoDuplicateValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = NoDuplicateValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object members")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Number(Number::from(value))))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Number(Number::from(value))))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(Value::Number)
                    .map(NoDuplicateValue)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::String(value.to_owned())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::String(value)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<NoDuplicateValue>()? {
                    values.push(value.0);
                }
                Ok(NoDuplicateValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = std::collections::BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let value = map.next_value::<NoDuplicateValue>()?.0;
                    if values.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate JSON object member"));
                    }
                }
                Ok(NoDuplicateValue(Value::Object(
                    values.into_iter().collect::<Map<_, _>>(),
                )))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

fn canonicalize_value(value: &Value, reserve: usize) -> Result<String, StagingSemanticDecodeError> {
    fn append(value: &Value, output: &mut String) -> Result<(), StagingSemanticDecodeError> {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => {
                let integer = value.as_i64().ok_or(StagingSemanticDecodeError::Shape(
                    "only integer numbers are allowed",
                ))?;
                if !(-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&integer) {
                    return Err(StagingSemanticDecodeError::Shape(
                        "number is outside the JCS safe-integer domain",
                    ));
                }
                output.push_str(&integer.to_string());
            }
            Value::String(value) => push_jcs_string(output, value),
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    append(value, output)?;
                }
                output.push(']');
            }
            Value::Object(values) => {
                output.push('{');
                let mut members: Vec<_> = values.iter().collect();
                members
                    .sort_by(|(left, _), (right, _)| left.encode_utf16().cmp(right.encode_utf16()));
                for (index, (key, value)) in members.into_iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    push_jcs_string(output, key);
                    output.push(':');
                    append(value, output)?;
                }
                output.push('}');
            }
        }
        Ok(())
    }
    let mut output = String::new();
    output
        .try_reserve(reserve)
        .map_err(|_| StagingSemanticDecodeError::Limit)?;
    append(value, &mut output)?;
    Ok(output)
}

#[cfg(feature = "staging-fixtures")]
#[doc(hidden)]
pub fn staging_math_document_body_fixture(
    input: &[u8],
) -> Result<Vec<u8>, StagingSemanticDecodeError> {
    fn shift_node_ids(value: &mut Value) -> Result<(), StagingSemanticDecodeError> {
        match value {
            Value::Object(object) => {
                if let Some(node_id) = object.get_mut("node_id") {
                    let shifted = node_id
                        .as_u64()
                        .and_then(|value| value.checked_sub(1))
                        .ok_or(StagingSemanticDecodeError::Shape(
                            "fixture node_id cannot be shifted",
                        ))?;
                    *node_id = Value::Number(Number::from(shifted));
                }
                for child in object.values_mut() {
                    shift_node_ids(child)?;
                }
            }
            Value::Array(values) => {
                for child in values {
                    shift_node_ids(child)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    let mut value: Value =
        serde_json::from_slice(input).map_err(StagingSemanticDecodeError::Json)?;
    let blocks = value
        .get_mut("document")
        .and_then(Value::as_object_mut)
        .and_then(|document| document.get_mut("blocks"))
        .and_then(Value::as_array_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "fixture document blocks are required",
        ))?;
    if blocks.len() != 1 {
        return Err(StagingSemanticDecodeError::Shape(
            "fixture must have one semantic-container root",
        ));
    }
    let mut container = blocks.remove(0);
    let container = container
        .as_object_mut()
        .filter(|object| object.get("kind").and_then(Value::as_str) == Some("semantic_container"))
        .ok_or(StagingSemanticDecodeError::Shape(
            "fixture root must be a semantic container",
        ))?;
    let mut children = container.remove("blocks").filter(Value::is_array).ok_or(
        StagingSemanticDecodeError::Shape("fixture semantic blocks are required"),
    )?;
    shift_node_ids(&mut children)?;
    value["document"]["blocks"] = children;
    let selector = value
        .get_mut("style_sheet")
        .and_then(Value::as_object_mut)
        .and_then(|sheet| sheet.get_mut("rules"))
        .and_then(Value::as_array_mut)
        .and_then(|rules| rules.first_mut())
        .and_then(Value::as_object_mut)
        .and_then(|rule| rule.get_mut("selector"))
        .ok_or(StagingSemanticDecodeError::Shape(
            "fixture style selector is required",
        ))?;
    *selector = Value::String("paragraph".to_owned());
    serde_json::to_vec(&value).map_err(StagingSemanticDecodeError::Json)
}

#[cfg(feature = "staging-fixtures")]
#[doc(hidden)]
pub fn staging_book_navigation_page_region_fixture(
    input: &[u8],
) -> Result<Vec<u8>, StagingSemanticDecodeError> {
    let mut value: Value =
        serde_json::from_slice(input).map_err(StagingSemanticDecodeError::Json)?;
    let master = value
        .get_mut("page_masters")
        .and_then(Value::as_object_mut)
        .and_then(|page_masters| page_masters.get_mut("masters"))
        .and_then(Value::as_array_mut)
        .and_then(|masters| masters.first_mut())
        .and_then(Value::as_object_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "fixture page master is required",
        ))?;
    master.insert(
        "header".to_owned(),
        serde_json::json!({
            "height": 1_638_400,
            "width": 52_428_800,
            "x": 6_553_600,
            "y": 3_276_800
        }),
    );
    master.insert(
        "header_content".to_owned(),
        serde_json::json!({
            "blocks": [{
                "children": [{
                    "kind": "text",
                    "node_id": 12,
                    "span": {"end_byte": 7, "source_id": 0, "start_byte": 0},
                    "text_span": {"end_byte": 7, "start_byte": 0, "text_id": 0}
                }],
                "classes": [],
                "kind": "paragraph",
                "node_id": 11,
                "span": {"end_byte": 7, "source_id": 0, "start_byte": 0}
            }],
            "node_id": 10,
            "span": {"end_byte": 7, "source_id": 0, "start_byte": 0}
        }),
    );
    serde_json::to_vec(&value).map_err(StagingSemanticDecodeError::Json)
}

#[cfg(feature = "staging-fixtures")]
#[doc(hidden)]
pub fn staging_book_navigation_wrong_parent_fixture(
    input: &[u8],
) -> Result<Vec<u8>, StagingSemanticDecodeError> {
    let mut value: Value =
        serde_json::from_slice(input).map_err(StagingSemanticDecodeError::Json)?;
    let parent = value
        .get_mut("outline")
        .and_then(Value::as_object_mut)
        .and_then(|outline| outline.get_mut("entries"))
        .and_then(Value::as_array_mut)
        .and_then(|entries| entries.get_mut(1))
        .and_then(Value::as_object_mut)
        .and_then(|entry| entry.get_mut("parent_outline_id"))
        .ok_or(StagingSemanticDecodeError::Shape(
            "fixture second outline parent is required",
        ))?;
    *parent = Value::Null;
    serde_json::to_vec(&value).map_err(StagingSemanticDecodeError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"
    ));
    const VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vector-media/job/document-package.json"
    ));
    const MATH_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json"
    ));
    const BOOK_NAVIGATION_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"
    ));
    const PRECOMPOSED_VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"
    ));

    fn policy() -> DocumentPackageDecodePolicy<'static> {
        let limits = Box::leak(Box::new(
            ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        ));
        DocumentPackageDecodePolicy::new(limits)
    }

    #[test]
    fn semantic_container_wire_round_trip_is_typed_and_canonical() {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &policy())
            .unwrap();
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(encoded, decoded.canonical_jcs());
        assert!(encoded.contains("\"semantic_kind\":\"result\""));
        assert!(encoded.contains("\"semantic_kind\":\"proof\""));
        assert!(encoded.contains("\"semantic_kind\":\"exercise\""));
        assert!(encoded.contains("\"media_type\":\"png\""));
        assert!(encoded.contains("\"media_type\":\"sfnt-truetype-glyf\""));
        assert!(encoded.contains("\"media_type\":\"ttc-truetype-glyf\""));
    }

    #[test]
    fn precomposed_vector_wire_round_trip_is_lossless_canonical_and_private() {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(PRECOMPOSED_VECTOR_FIXTURE, &policy())
            .unwrap();
        let wire = decoded.wire();
        assert_eq!(wire.resources().images.len(), 1);
        let image = &wire.resources().images[0];
        assert_eq!(image.media_type, WireImageMediaType::SvgSafe2);
        assert!(image.expected_sha256.is_some());
        assert_eq!(
            image
                .vector_provenance
                .as_ref()
                .map(|value| value.engine_id.as_str()),
            Some("vmb.texToSvg")
        );

        let WireStagingM4Block::SemanticContainer { blocks, .. } = &wire.document().blocks[0]
        else {
            panic!("fixture root must be a semantic container");
        };
        let WireStagingM4Block::Paragraph { children, .. } = &blocks[0] else {
            panic!("first child must be a paragraph");
        };
        assert!(matches!(
            &children[0],
            WireStagingM4Inline::InlineVector {
                actual_text: None,
                ..
            }
        ));
        let WireStagingM4Inline::MathVector {
            metrics,
            source_tex,
            ..
        } = &children[1]
        else {
            panic!("second inline must be math_vector");
        };
        assert_eq!(metrics.advance, 2_031_616);
        assert_eq!(metrics.viewport.width, 1_966_080);
        assert_eq!(source_tex.text_span.start_byte, 3);
        assert!(matches!(blocks[1], WireStagingM4Block::VectorFigure { .. }));
        let WireStagingM4Block::MathVectorBlock {
            equation_number: Some(number),
            source_tex,
            ..
        } = &blocks[2]
        else {
            panic!("third child must be numbered math_vector_block");
        };
        assert_eq!(number.node_id, 7);
        assert_eq!(number.minimum_gap, 65_536);
        assert_eq!(source_tex.text_span.start_byte, 7);

        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(wire)
            .unwrap();
        assert_eq!(encoded, decoded.canonical_jcs());
        assert_eq!(
            PRECOMPOSED_VECTOR_FIXTURE.strip_suffix(b"\n").unwrap(),
            encoded.as_bytes()
        );
        assert!(crate::StrictDocumentPackageDecoder::new()
            .decode(PRECOMPOSED_VECTOR_FIXTURE, &policy())
            .is_err());
    }

    #[test]
    fn precomposed_vector_wire_conditional_errors_have_exact_p1102_pointers() {
        fn rejected(value: &Value, pointer: &str) {
            let error = StagingSemanticDocumentPackageDecoder::new()
                .decode(&serde_json::to_vec(value).unwrap(), &policy())
                .unwrap_err();
            assert_eq!(error.pointer(), Some(pointer));
            assert!(error.to_string().starts_with("P1102:"));
        }

        let fixture: Value = serde_json::from_slice(PRECOMPOSED_VECTOR_FIXTURE).unwrap();

        let mut missing_source = fixture.clone();
        missing_source["document"]["blocks"][0]["blocks"][0]["children"][1]
            .as_object_mut()
            .unwrap()
            .remove("source_tex");
        rejected(
            &missing_source,
            "/document/blocks/0/blocks/0/children/1/source_tex",
        );

        let mut wrong_source_text_id = fixture.clone();
        wrong_source_text_id["document"]["blocks"][0]["blocks"][0]["children"][1]["source_tex"]
            ["text_span"]["text_id"] = Value::String("0".to_owned());
        rejected(
            &wrong_source_text_id,
            "/document/blocks/0/blocks/0/children/1/source_tex/text_span/text_id",
        );

        let mut forbidden_source = fixture.clone();
        forbidden_source["document"]["blocks"][0]["blocks"][0]["children"][0]["source_tex"] =
            serde_json::json!({"text_span":{"end_byte":3,"start_byte":0,"text_id":0}});
        rejected(
            &forbidden_source,
            "/document/blocks/0/blocks/0/children/0/source_tex",
        );

        let mut escaped_unknown_member = fixture.clone();
        escaped_unknown_member["document"]["blocks"][0]["blocks"][0]["children"][0]
            .as_object_mut()
            .unwrap()
            .insert("source/te~x".to_owned(), Value::Null);
        rejected(
            &escaped_unknown_member,
            "/document/blocks/0/blocks/0/children/0/source~1te~0x",
        );

        let mut missing_actual = fixture.clone();
        missing_actual["document"]["blocks"][0]["blocks"][2]
            .as_object_mut()
            .unwrap()
            .remove("actual_text");
        rejected(&missing_actual, "/document/blocks/0/blocks/2/actual_text");

        let mut missing_equation = fixture.clone();
        missing_equation["document"]["blocks"][0]["blocks"][2]
            .as_object_mut()
            .unwrap()
            .remove("equation_number");
        rejected(
            &missing_equation,
            "/document/blocks/0/blocks/2/equation_number",
        );

        let mut wrong_equation_span = fixture.clone();
        wrong_equation_span["document"]["blocks"][0]["blocks"][2]["equation_number"]["span"]
            ["end_byte"] = Value::String("14".to_owned());
        rejected(
            &wrong_equation_span,
            "/document/blocks/0/blocks/2/equation_number/span/end_byte",
        );

        let mut zero_advance = fixture.clone();
        zero_advance["document"]["blocks"][0]["blocks"][0]["children"][0]["metrics"]["advance"] =
            Value::from(0);
        rejected(
            &zero_advance,
            "/document/blocks/0/blocks/0/children/0/metrics/advance",
        );

        let mut null_hash = fixture.clone();
        null_hash["resources"]["images"][0]["expected_sha256"] = Value::Null;
        rejected(&null_hash, "/resources/images/0/expected_sha256");

        let mut missing_provenance = fixture.clone();
        missing_provenance["resources"]["images"][0]
            .as_object_mut()
            .unwrap()
            .remove("vector_provenance");
        rejected(&missing_provenance, "/resources/images/0/vector_provenance");

        let mut invalid_provenance = fixture.clone();
        invalid_provenance["resources"]["images"][0]["vector_provenance"]["engine_id"] =
            Value::String("vmb\ntexToSvg".to_owned());
        rejected(
            &invalid_provenance,
            "/resources/images/0/vector_provenance/engine_id",
        );

        let mut forbidden_provenance = fixture.clone();
        forbidden_provenance["resources"]["images"][0]["media_type"] =
            Value::String("svg-safe-1".to_owned());
        rejected(
            &forbidden_provenance,
            "/resources/images/0/vector_provenance",
        );

        let mut duplicate_image = fixture;
        let duplicate = duplicate_image["resources"]["images"][0].clone();
        duplicate_image["resources"]["images"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        rejected(&duplicate_image, "/resources/images/1/image_id");
    }

    #[test]
    fn precomposed_vector_wire_rejects_duplicate_unknown_wrong_and_extra_shapes() {
        let duplicate = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replacen(
                "\"alt\":\"丸括弧で囲んだ二項目\"",
                "\"\\u0061lt\":\"duplicate\",\"alt\":\"丸括弧で囲んだ二項目\"",
                1,
            );
        assert!(matches!(
            StagingSemanticDocumentPackageDecoder::new().decode(duplicate.as_bytes(), &policy()),
            Err(StagingSemanticDecodeError::Json(_))
                | Err(StagingSemanticDecodeError::Preflight(_))
        ));

        let fixture: Value = serde_json::from_slice(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        let mut unknown_kind = fixture.clone();
        unknown_kind["document"]["blocks"][0]["blocks"][0]["children"][0]["kind"] =
            Value::String("unknown_vector".to_owned());
        let unknown_error = StagingSemanticDocumentPackageDecoder::new()
            .decode(&serde_json::to_vec(&unknown_kind).unwrap(), &policy())
            .unwrap_err();
        assert_eq!(
            unknown_error.pointer(),
            Some("/document/blocks/0/blocks/0/children/0/kind")
        );
        assert!(unknown_error.to_string().starts_with("P1102:"));
        let mut wrong_type = fixture.clone();
        wrong_type["document"]["blocks"][0]["blocks"][0]["children"][0]["metrics"] =
            Value::String("not-metrics".to_owned());
        let mut extra_member = fixture;
        extra_member["document"]["blocks"][0]["blocks"][1]["source_tex"] =
            serde_json::json!({"text_span":{"end_byte":7,"start_byte":6,"text_id":0}});
        for invalid in [wrong_type, extra_member] {
            assert!(StagingSemanticDocumentPackageDecoder::new()
                .decode(&serde_json::to_vec(&invalid).unwrap(), &policy())
                .is_err());
        }
    }

    #[test]
    fn book_navigation_wire_round_trip_is_typed_closed_and_private() {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(BOOK_NAVIGATION_FIXTURE, &policy())
            .unwrap();
        assert_eq!(decoded.wire().document().language, "en-US");
        assert_eq!(
            decoded.wire().metadata().title.as_deref(),
            Some("Typaxis Book")
        );
        assert_eq!(decoded.wire().outline().entries.len(), 3);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(encoded, decoded.canonical_jcs());
        assert!(encoded.contains("\"language\":\"FR-latn-fr\""));
        assert!(encoded.contains("\"anchor_id\":\"part-1\""));
        assert!(crate::StrictDocumentPackageDecoder::new()
            .decode(BOOK_NAVIGATION_FIXTURE, &policy())
            .is_err());

        let mut missing_metadata: Value = serde_json::from_slice(BOOK_NAVIGATION_FIXTURE).unwrap();
        missing_metadata.as_object_mut().unwrap().remove("metadata");
        let missing_error = StagingSemanticDocumentPackageDecoder::new()
            .decode(&serde_json::to_vec(&missing_metadata).unwrap(), &policy())
            .unwrap_err();
        assert_eq!(missing_error.pointer(), Some("/metadata"));
        assert!(missing_error.to_string().starts_with("P1102:"));

        let mut null_override: Value = serde_json::from_slice(BOOK_NAVIGATION_FIXTURE).unwrap();
        null_override["document"]["blocks"][0]["blocks"][0]["language"] = Value::Null;
        let null_error = StagingSemanticDocumentPackageDecoder::new()
            .decode(&serde_json::to_vec(&null_override).unwrap(), &policy())
            .unwrap_err();
        assert_eq!(
            null_error.pointer(),
            Some("/document/blocks/0/blocks/0/language")
        );

        let mut wrong_outline: Value = serde_json::from_slice(BOOK_NAVIGATION_FIXTURE).unwrap();
        wrong_outline["outline"]["entries"][1]["source_node_id"] = Value::String("2".into());
        let outline_error = StagingSemanticDocumentPackageDecoder::new()
            .decode(&serde_json::to_vec(&wrong_outline).unwrap(), &policy())
            .unwrap_err();
        assert_eq!(
            outline_error.pointer(),
            Some("/outline/entries/1/source_node_id")
        );

        let mut ordered_outline: Value = serde_json::from_slice(BOOK_NAVIGATION_FIXTURE).unwrap();
        ordered_outline["outline"]["entries"][1]["parent_outline_id"] = Value::String("0".into());
        ordered_outline["outline"]["entries"][1]["source_node_id"] = Value::String("2".into());
        let ordered_error = StagingSemanticDocumentPackageDecoder::new()
            .decode(&serde_json::to_vec(&ordered_outline).unwrap(), &policy())
            .unwrap_err();
        assert_eq!(
            ordered_error.pointer(),
            Some("/outline/entries/1/parent_outline_id")
        );
    }

    #[test]
    fn book_navigation_wire_combined_ast_limit_is_inclusive() {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(BOOK_NAVIGATION_FIXTURE, &policy())
            .unwrap();
        let wire = decoded.wire();
        let navigation_nodes = 2u64
            + u64::try_from(wire.metadata().keywords.len()).unwrap()
            + u64::try_from(wire.outline().entries.len()).unwrap();
        let exact_nodes = document_node_count(
            wire.document(),
            ResourceLimits::default().max_ast_nesting_depth,
        )
        .unwrap()
        .checked_add(advanced_page_node_count(wire.advanced_page_masters()).unwrap())
        .unwrap()
        .checked_add(navigation_nodes)
        .unwrap();

        let exact = ResourceLimits {
            max_ast_nodes: exact_nodes,
            ..ResourceLimits::default()
        };
        let exact = ValidatedResourceLimits::new(exact).unwrap();
        StagingSemanticDocumentPackageDecoder::new()
            .decode(
                BOOK_NAVIGATION_FIXTURE,
                &DocumentPackageDecodePolicy::new(&exact),
            )
            .unwrap();

        let below = ResourceLimits {
            max_ast_nodes: exact_nodes - 1,
            ..ResourceLimits::default()
        };
        let below = ValidatedResourceLimits::new(below).unwrap();
        assert!(matches!(
            StagingSemanticDocumentPackageDecoder::new().decode(
                BOOK_NAVIGATION_FIXTURE,
                &DocumentPackageDecodePolicy::new(&below),
            ),
            Err(StagingSemanticDecodeError::Limit)
        ));
    }

    #[test]
    fn vector_media_wire_round_trip_is_private_closed_and_typed() {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(VECTOR_FIXTURE, &policy())
            .unwrap();
        assert_eq!(decoded.wire().resources().images.len(), 2);
        assert_eq!(decoded.wire().page_masters().masters.len(), 1);
        assert_eq!(decoded.wire().page_masters().masters[0].width, 65_536_000);
        assert_eq!(
            decoded.wire().resources().images[0].media_type,
            WireImageMediaType::SvgSafe1
        );
        assert_eq!(
            decoded.wire().resources().images[1].media_type,
            WireImageMediaType::SvgSafe1
        );
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(encoded, decoded.canonical_jcs());
        assert!(encoded.contains("\"media_type\":\"svg-safe-1\""));

        let unknown = String::from_utf8(VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replacen("svg-safe-1", "image/svg+xml", 1);
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(unknown.as_bytes(), &policy())
            .is_err());
        assert!(crate::StrictDocumentPackageDecoder::new()
            .decode(VECTOR_FIXTURE, &policy())
            .is_err());
        assert_eq!(
            typaxis_core::DocumentPackageContractId::CURRENT.as_str(),
            "typaxis.contract/1.3"
        );
    }

    #[test]
    fn math_wire_round_trip_is_typed_versioned_and_private() {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(MATH_FIXTURE, &policy())
            .unwrap();
        let WireStagingM4Block::SemanticContainer { blocks, .. } =
            &decoded.wire().document().blocks[0]
        else {
            panic!("math fixture root must be a semantic container")
        };
        let WireStagingM4Block::Paragraph { children, .. } = &blocks[0] else {
            panic!("first math owner must be a paragraph")
        };
        let WireStagingM4Inline::InlineMath {
            math_source,
            speech,
            ..
        } = &children[0]
        else {
            panic!("paragraph child must remain inline math")
        };
        assert_eq!(math_source.language, "typaxis-math");
        assert_eq!(math_source.version, "1");
        assert_eq!(math_source.text_span.start_byte, 0);
        assert_eq!(math_source.text_span.end_byte, 5);
        assert_eq!(speech, "x squared");
        let WireStagingM4Block::DisplayMath {
            math_source,
            speech,
            ..
        } = &blocks[1]
        else {
            panic!("second math owner must remain display math")
        };
        assert_eq!(math_source.text_span.start_byte, 5);
        assert_eq!(math_source.text_span.end_byte, 8);
        assert_eq!(speech, "x plus one");
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(encoded, decoded.canonical_jcs());
        assert!(crate::StrictDocumentPackageDecoder::new()
            .decode(MATH_FIXTURE, &policy())
            .is_err());

        let wrong_version = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "\"version\":\"1\"",
            "\"version\":\"2\"",
            1,
        );
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(wrong_version.as_bytes(), &policy())
            .is_err());
    }

    #[test]
    fn semantic_container_unknown_kind_and_missing_media_are_rejected() {
        let unknown = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"semantic_kind\":\"result\"",
            "\"semantic_kind\":\"lemma\"",
            1,
        );
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(unknown.as_bytes(), &policy())
            .is_err());

        let missing =
            String::from_utf8(FIXTURE.to_vec())
                .unwrap()
                .replacen("\"media_type\":\"png\",", "", 1);
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(missing.as_bytes(), &policy())
            .is_err());
    }

    #[test]
    fn semantic_container_structural_empty_is_rejected_by_decoder_and_encoder() {
        let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
        value["document"]["blocks"][0]["blocks"] = Value::Array(Vec::new());
        let empty = serde_json::to_vec(&value).unwrap();
        assert!(matches!(
            StagingSemanticDocumentPackageDecoder::new().decode(&empty, &policy()),
            Err(StagingSemanticDecodeError::Shape(
                "semantic_container blocks must not be empty"
            ))
        ));

        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &policy())
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
            panic!("fixture root block must remain a semantic container")
        };
        blocks.clear();
        wire.replace_typed_regions(document, wire.resources().clone());
        assert!(matches!(
            StagingSemanticDocumentPackageEncoder::new().encode(&wire),
            Err(StagingSemanticDecodeError::Shape(
                "semantic_container blocks must not be empty"
            ))
        ));

        let inline = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"kind\":\"text\"",
            "\"kind\":\"semantic_container\"",
            1,
        );
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(inline.as_bytes(), &policy())
            .is_err());
    }

    #[test]
    fn semantic_container_contract_is_not_accepted_by_the_public_decoder() {
        assert!(crate::StrictDocumentPackageDecoder::new()
            .decode(FIXTURE, &policy())
            .is_err());
        assert_eq!(
            typaxis_core::DocumentPackageContractId::CURRENT.as_str(),
            "typaxis.contract/1.3"
        );
    }

    #[test]
    fn semantic_container_encoder_reapplies_the_receipted_decode_limits() {
        let raw_limits = ResourceLimits {
            max_document_package_bytes: u64::try_from(FIXTURE.len()).unwrap(),
            ..ResourceLimits::default()
        };
        let limits = ValidatedResourceLimits::new(raw_limits).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        let WireStagingM4Block::SemanticContainer { classes, .. } = &mut document.blocks[0] else {
            panic!("fixture root block must remain a semantic container")
        };
        classes.push("growth".to_owned());
        let resources = wire.resources().clone();
        wire.replace_typed_regions(document, resources);
        assert!(matches!(
            StagingSemanticDocumentPackageEncoder::new().encode(&wire),
            Err(StagingSemanticDecodeError::Limit)
        ));
    }
}
