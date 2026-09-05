use std::sync::Arc;

use typaxis_core::{AnchorId, NodeId, SourceSpan};

/// Lossless contract-1.4 metadata domain. Validation authority belongs to the
/// syntax-owned `DocumentMetadataReceipt`; this value alone is not a receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDocumentMetadata {
    pub author: Option<String>,
    pub created: Option<String>,
    pub identifier: Option<String>,
    pub keywords: Vec<String>,
    pub modified: Option<String>,
    pub subject: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingLanguageNodeKind {
    Document,
    SemanticContainer,
    Paragraph,
    Heading,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    Figure,
    FootnoteDefinition,
    Text,
    Emphasis,
    Strong,
    Link,
    Reference,
    FootnoteReference,
    InlineMath,
    DisplayMath,
    InlineVector,
    MathVector,
    VectorFigure,
    MathVectorBlock,
}

impl StagingLanguageNodeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::SemanticContainer => "semantic_container",
            Self::Paragraph => "paragraph",
            Self::Heading => "heading",
            Self::List => "list",
            Self::ListItem => "list_item",
            Self::Table => "table",
            Self::TableRow => "table_row",
            Self::TableCell => "table_cell",
            Self::Figure => "figure",
            Self::FootnoteDefinition => "footnote_definition",
            Self::Text => "text",
            Self::Emphasis => "emphasis",
            Self::Strong => "strong",
            Self::Link => "link",
            Self::Reference => "reference",
            Self::FootnoteReference => "footnote_reference",
            Self::InlineMath => "inline_math",
            Self::DisplayMath => "display_math",
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
            Self::VectorFigure => "vector_figure",
            Self::MathVectorBlock => "math_vector_block",
        }
    }
}

/// Closed logical-owner vocabulary for the contract-1.4 computed
/// language registry `/2`.
///
/// This is nominally distinct from [`StagingLanguageNodeKind`] so the frozen
/// `/1` registry cannot be substituted at a `/2` boundary. The four
/// precomposed-vector variants are language owners; equation-number text is
/// deliberately not an owner.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingComputedLanguageOwnerKindV2 {
    Document,
    SemanticContainer,
    Paragraph,
    Heading,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    Figure,
    FootnoteDefinition,
    Text,
    Emphasis,
    Strong,
    Link,
    Reference,
    FootnoteReference,
    InlineMath,
    DisplayMath,
    InlineVector,
    MathVector,
    VectorFigure,
    MathVectorBlock,
}

impl StagingComputedLanguageOwnerKindV2 {
    pub const ALL: [Self; 23] = [
        Self::Document,
        Self::SemanticContainer,
        Self::Paragraph,
        Self::Heading,
        Self::List,
        Self::ListItem,
        Self::Table,
        Self::TableRow,
        Self::TableCell,
        Self::Figure,
        Self::FootnoteDefinition,
        Self::Text,
        Self::Emphasis,
        Self::Strong,
        Self::Link,
        Self::Reference,
        Self::FootnoteReference,
        Self::InlineMath,
        Self::DisplayMath,
        Self::InlineVector,
        Self::MathVector,
        Self::VectorFigure,
        Self::MathVectorBlock,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::SemanticContainer => "semantic_container",
            Self::Paragraph => "paragraph",
            Self::Heading => "heading",
            Self::List => "list",
            Self::ListItem => "list_item",
            Self::Table => "table",
            Self::TableRow => "table_row",
            Self::TableCell => "table_cell",
            Self::Figure => "figure",
            Self::FootnoteDefinition => "footnote_definition",
            Self::Text => "text",
            Self::Emphasis => "emphasis",
            Self::Strong => "strong",
            Self::Link => "link",
            Self::Reference => "reference",
            Self::FootnoteReference => "footnote_reference",
            Self::InlineMath => "inline_math",
            Self::DisplayMath => "display_math",
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
            Self::VectorFigure => "vector_figure",
            Self::MathVectorBlock => "math_vector_block",
        }
    }

    pub const fn is_precomposed_vector(self) -> bool {
        matches!(
            self,
            Self::InlineVector | Self::MathVector | Self::VectorFigure | Self::MathVectorBlock
        )
    }
}

impl From<StagingLanguageNodeKind> for StagingComputedLanguageOwnerKindV2 {
    fn from(value: StagingLanguageNodeKind) -> Self {
        match value {
            StagingLanguageNodeKind::Document => Self::Document,
            StagingLanguageNodeKind::SemanticContainer => Self::SemanticContainer,
            StagingLanguageNodeKind::Paragraph => Self::Paragraph,
            StagingLanguageNodeKind::Heading => Self::Heading,
            StagingLanguageNodeKind::List => Self::List,
            StagingLanguageNodeKind::ListItem => Self::ListItem,
            StagingLanguageNodeKind::Table => Self::Table,
            StagingLanguageNodeKind::TableRow => Self::TableRow,
            StagingLanguageNodeKind::TableCell => Self::TableCell,
            StagingLanguageNodeKind::Figure => Self::Figure,
            StagingLanguageNodeKind::FootnoteDefinition => Self::FootnoteDefinition,
            StagingLanguageNodeKind::Text => Self::Text,
            StagingLanguageNodeKind::Emphasis => Self::Emphasis,
            StagingLanguageNodeKind::Strong => Self::Strong,
            StagingLanguageNodeKind::Link => Self::Link,
            StagingLanguageNodeKind::Reference => Self::Reference,
            StagingLanguageNodeKind::FootnoteReference => Self::FootnoteReference,
            StagingLanguageNodeKind::InlineMath => Self::InlineMath,
            StagingLanguageNodeKind::DisplayMath => Self::DisplayMath,
            StagingLanguageNodeKind::InlineVector => Self::InlineVector,
            StagingLanguageNodeKind::MathVector => Self::MathVector,
            StagingLanguageNodeKind::VectorFigure => Self::VectorFigure,
            StagingLanguageNodeKind::MathVectorBlock => Self::MathVectorBlock,
        }
    }
}

/// Vector-only semantic facts joined to a computed-language owner. These
/// hashes bind Alt and resolved ActualText to the same effective language
/// without copying either authored string into downstream paint plans.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingVectorLanguageBindingV2 {
    pub metrics_fingerprint: [u8; 32],
    pub effective_language_fingerprint: [u8; 32],
    pub alternative_sha256: [u8; 32],
    pub resolved_actual_text_sha256: Option<[u8; 32]>,
    pub authored_language_charge_bytes: u64,
}

/// One source-preorder logical owner in computed-language registry `/2`.
/// `record_fingerprint` is the stable join key used by selected paint and the
/// later tagged-structure plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingComputedLanguageRecordV2 {
    pub node_id: NodeId,
    pub node_kind: StagingComputedLanguageOwnerKindV2,
    pub logical_parent_node_id: Option<NodeId>,
    pub source_span: Option<SourceSpan>,
    pub explicit_language: Option<Arc<str>>,
    pub effective_language: Arc<str>,
    pub language_text_charge_bytes: u64,
    pub vector_binding: Option<StagingVectorLanguageBindingV2>,
    pub record_fingerprint: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingComputedLanguageChildKindV2 {
    EquationNumber,
}

impl StagingComputedLanguageChildKindV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EquationNumber => "equation_number",
        }
    }
}

/// A language-bearing logical child that is not itself a registry owner.
/// Equation-number text inherits from its `math_vector_block` owner and joins
/// that owner's record fingerprint exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingComputedLanguageChildRecordV2 {
    pub node_id: NodeId,
    pub child_kind: StagingComputedLanguageChildKindV2,
    pub parent_owner_node_id: NodeId,
    pub source_span: SourceSpan,
    pub effective_language: Arc<str>,
    pub parent_language_record_fingerprint: [u8; 32],
    pub record_fingerprint: [u8; 32],
}

/// One lowered logical-owner language fact. The syntax receipt validates the
/// tag and inheritance and hashes a complete NodeId-ordered collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingComputedLanguageRecord {
    pub node_id: NodeId,
    pub node_kind: StagingLanguageNodeKind,
    pub logical_parent_node_id: Option<NodeId>,
    pub source_span: Option<SourceSpan>,
    pub explicit_language: Option<Arc<str>>,
    pub effective_language: Arc<str>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingOutlineSourceKind {
    Heading,
    SemanticContainer,
}

impl StagingOutlineSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::SemanticContainer => "semantic_container",
        }
    }
}

/// Source-owner proof retained by validated outline entries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingOutlineSource {
    pub kind: StagingOutlineSourceKind,
    pub node_id: NodeId,
    pub source_span: SourceSpan,
    pub anchor_id: AnchorId,
    pub heading_level: Option<u8>,
    pub semantic_kind: Option<String>,
    pub computed_language: String,
}

/// Canonical outline-domain entry. Its containing syntax receipt proves dense
/// IDs, preorder, stack parentage, owner/anchor equality, and uniqueness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingOutlineEntry {
    pub outline_id: u32,
    pub parent_outline_id: Option<u32>,
    pub level: u8,
    pub destination: AnchorId,
    pub label: String,
    pub source: StagingOutlineSource,
}
