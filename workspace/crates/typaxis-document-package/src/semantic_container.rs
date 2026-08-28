//! Private contract-1.4 carrier for MI4 staging. The public strict decoder and
//! current aliases remain on contract 1.3 until MI4-13.

use crate::{
    DocumentPackageDecodePolicy, JsonPreflightError, StrictJsonPreflight, WirePageMasterSet,
    WireSourceSpan,
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
}

impl WireImageMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::SvgSafe1 => "svg-safe-1",
        }
    }
}

impl<'de> Deserialize<'de> for WireImageMediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "png" => Ok(Self::Png),
            "svg-safe-1" => Ok(Self::SvgSafe1),
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireStagingM4Inline {
    Text {
        node_id: u32,
        span: WireStagingSourceSpan,
        text_span: WireStagingTextSpan,
    },
    InlineMath {
        node_id: u32,
        span: WireStagingSourceSpan,
        math_source: WireStagingMathSource,
        speech: String,
    },
    Emphasis {
        node_id: u32,
        span: WireStagingSourceSpan,
        children: Vec<WireStagingM4Inline>,
    },
    Strong {
        node_id: u32,
        span: WireStagingSourceSpan,
        children: Vec<WireStagingM4Inline>,
    },
    Link {
        node_id: u32,
        span: WireStagingSourceSpan,
        target: WireStagingM4LinkTarget,
        children: Vec<WireStagingM4Inline>,
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
    },
    FootnoteReference {
        node_id: u32,
        span: WireStagingSourceSpan,
        footnote_id: String,
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4ListItem {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub blocks: Vec<WireStagingM4Block>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4TableCell {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub colspan: u16,
    pub rowspan: u16,
    pub blocks: Vec<WireStagingM4Block>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4TableRow {
    pub node_id: u32,
    pub span: WireStagingSourceSpan,
    pub cells: Vec<WireStagingM4TableCell>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireStagingM4Block {
    Paragraph {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        children: Vec<WireStagingM4Inline>,
    },
    Heading {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        level: u8,
        anchor_id: Option<String>,
        children: Vec<WireStagingM4Inline>,
    },
    List {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        ordered: bool,
        start: Option<u32>,
        items: Vec<WireStagingM4ListItem>,
    },
    Table {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        columns: Vec<Value>,
        head: Vec<WireStagingM4TableRow>,
        body: Vec<WireStagingM4TableRow>,
    },
    Figure {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        image_id: u32,
        placement: String,
        alt: String,
        caption: Vec<WireStagingM4Block>,
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
    },
    SemanticContainer {
        node_id: u32,
        span: WireStagingSourceSpan,
        classes: Vec<String>,
        semantic_kind: WireStagingSemanticContainerKind,
        blocks: Vec<WireStagingM4Block>,
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
            | Self::SemanticContainer { classes, .. } => classes,
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireStagingM4Document {
    pub node_id: u32,
    pub blocks: Vec<WireStagingM4Block>,
    pub footnotes: Vec<WireStagingM4Footnote>,
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
    resources: WireStagingM4ResourceCatalog,
    sources: Vec<WireStagingM4Source>,
    style_sheet: WireStagingStyleSheet,
    text_buffers: Vec<WireStagingM4TextBuffer>,
    page_masters: WirePageMasterSet,
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
    Limit,
}

impl fmt::Display for StagingSemanticDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "invalid contract-1.4 JSON: {error}"),
            Self::Contract => formatter.write_str("expected private typaxis.contract/1.4"),
            Self::Shape(message) => write!(formatter, "invalid contract-1.4 shape: {message}"),
            Self::Limit => formatter.write_str("contract-1.4 package exceeds a resource limit"),
        }
    }
}

impl std::error::Error for StagingSemanticDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Contract | Self::Shape(_) | Self::Limit => None,
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
        let expected: BTreeSet<&str> = [
            "contract",
            "coordinate_unit",
            "document",
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
        if object.get("contract").and_then(Value::as_str)
            != Some(STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT)
        {
            return Err(StagingSemanticDecodeError::Contract);
        }
        if object.get("coordinate_unit").and_then(Value::as_str) != Some("pdf_point_1_65536") {
            return Err(StagingSemanticDecodeError::Shape(
                "coordinate_unit must be pdf_point_1_65536",
            ));
        }
        let page_masters = validate_frozen_carrier(&root, policy)?;
        let document: WireStagingM4Document = serde_json::from_value(
            object
                .get("document")
                .cloned()
                .ok_or(StagingSemanticDecodeError::Shape("document is required"))?,
        )
        .map_err(StagingSemanticDecodeError::Json)?;
        validate_semantic_container_shape(&document)?;
        validate_math_wire(&document)?;
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
        if document_node_count(
            &document,
            policy.resource_limits().get().max_ast_nesting_depth,
        )? > policy.resource_limits().get().max_ast_nodes
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
                resources,
                sources,
                style_sheet,
                text_buffers,
                page_masters,
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
) -> Result<WirePageMasterSet, StagingSemanticDecodeError> {
    let mut compatibility = root.clone();
    let object = compatibility
        .as_object_mut()
        .ok_or(StagingSemanticDecodeError::Shape("root must be an object"))?;
    object.insert(
        "contract".to_owned(),
        Value::String("typaxis.contract/1.3".to_owned()),
    );
    let document = object
        .get_mut("document")
        .and_then(Value::as_object_mut)
        .ok_or(StagingSemanticDecodeError::Shape(
            "document must be an object",
        ))?;
    flatten_semantic_blocks(document.get_mut("blocks").ok_or(
        StagingSemanticDecodeError::Shape("document blocks are required"),
    )?)?;
    if let Some(footnotes) = document.get_mut("footnotes").and_then(Value::as_array_mut) {
        for footnote in footnotes {
            let blocks = footnote
                .as_object_mut()
                .and_then(|footnote| footnote.get_mut("blocks"))
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
        }
    }
    let bytes = canonicalize_value(&compatibility, 0)?;
    let decoded = crate::StagingAdvancedDocumentPackageDecoder::new()
        .decode(bytes.as_bytes(), policy)
        .map_err(|_| {
            StagingSemanticDecodeError::Shape("unchanged contract-1.3 carrier is invalid")
        })?;
    let (wire, _, _, _, _, _) = decoded.into_parts();
    Ok(wire.page_masters)
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
                    let children = item
                        .as_object_mut()
                        .and_then(|item| item.get_mut("blocks"))
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
                        let cells = row
                            .as_object_mut()
                            .and_then(|row| row.get_mut("cells"))
                            .and_then(Value::as_array_mut)
                            .ok_or(StagingSemanticDecodeError::Shape(
                                "table cells are required",
                            ))?;
                        for cell in cells {
                            let children = cell
                                .as_object_mut()
                                .and_then(|cell| cell.get_mut("blocks"))
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
                WireStagingM4Block::Figure { caption, .. } => visit(caption)?,
                WireStagingM4Block::Paragraph { .. }
                | WireStagingM4Block::Heading { .. }
                | WireStagingM4Block::DisplayMath { .. }
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
        let node_count = document_node_count(&package.document, limits.max_ast_nesting_depth)?;
        if node_count > limits.max_ast_nodes
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
                    Some("semantic_container" | "display_math" | "inline_math")
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
                | WireStagingM4Block::SemanticContainer {
                    blocks: caption, ..
                } => blocks(caption)?,
                WireStagingM4Block::DisplayMath { math_source, .. } => source(math_source)?,
                WireStagingM4Block::PageBreak { .. } => {}
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
                WireStagingM4Block::Table { head, body, .. } => {
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
            WireStagingM4Inline::InlineMath { .. } => {}
            _ => {}
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
