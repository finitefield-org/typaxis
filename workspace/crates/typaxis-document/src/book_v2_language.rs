//! Closed successor language ownership. No conversion into a legacy registry.
use crate::StagingLanguageNodeKind;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BookV2LanguageNodeKind {
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
    DescriptionList,
    DescriptionItem,
    DescriptionTerm,
}

impl BookV2LanguageNodeKind {
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
            Self::DescriptionList => "description_list",
            Self::DescriptionItem => "description_item",
            Self::DescriptionTerm => "description_term",
        }
    }
    pub const fn is_precomposed_vector(self) -> bool {
        matches!(
            self,
            Self::InlineVector | Self::MathVector | Self::VectorFigure | Self::MathVectorBlock
        )
    }
}

impl From<StagingLanguageNodeKind> for BookV2LanguageNodeKind {
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
