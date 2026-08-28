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
        }
    }
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
