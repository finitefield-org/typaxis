use core::num::NonZeroU16;
use typaxis_core::{FontFaceId, ImageResourceId, NodeId, PortablePath, SourceSpan};

/// Closed contract-1.4 semantic-container vocabulary adopted by ADR-0032.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticContainerKind {
    Result,
    Proof,
    Exercise,
}

impl SemanticContainerKind {
    pub const ALL: [Self; 3] = [Self::Result, Self::Proof, Self::Exercise];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Result => "result",
            Self::Proof => "proof",
            Self::Exercise => "exercise",
        }
    }
}

/// Closed image declarations initially supported by the private 1.4 registry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ImageMediaType {
    Png,
}

impl ImageMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
        }
    }
}

/// Closed TrueType-outline containers initially supported by 1.4 staging.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FontMediaType {
    SfntTrueTypeGlyf,
    TtcTrueTypeGlyf,
}

impl FontMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SfntTrueTypeGlyf => "sfnt-truetype-glyf",
            Self::TtcTrueTypeGlyf => "ttc-truetype-glyf",
        }
    }
}

/// Version-bound compatibility representation. A frozen contract lowers to
/// `LegacyUnspecified`; contract 1.4 can only lower to `Declared`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageMediaDeclaration {
    LegacyUnspecified,
    Declared(ImageMediaType),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontMediaDeclaration {
    LegacyUnspecified,
    Declared(FontMediaType),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4BlockCommon {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4ListItem {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub blocks: Vec<StagingM4Block>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4TableCell {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub colspan: NonZeroU16,
    pub rowspan: NonZeroU16,
    pub blocks: Vec<StagingM4Block>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4TableRow {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub cells: Vec<StagingM4TableCell>,
}

/// Private contract-1.4 block domain. It deliberately does not add a variant
/// to the frozen public `Block` enum before MI4-13.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingM4Block {
    Paragraph {
        common: StagingM4BlockCommon,
        has_authored_content: bool,
    },
    Heading {
        common: StagingM4BlockCommon,
        has_authored_content: bool,
    },
    List {
        common: StagingM4BlockCommon,
        items: Vec<StagingM4ListItem>,
    },
    Table {
        common: StagingM4BlockCommon,
        head: Vec<StagingM4TableRow>,
        body: Vec<StagingM4TableRow>,
    },
    Figure {
        common: StagingM4BlockCommon,
        has_nonempty_alternative: bool,
        caption: Vec<StagingM4Block>,
    },
    PageBreak {
        common: StagingM4BlockCommon,
    },
    SemanticContainer {
        common: StagingM4BlockCommon,
        semantic_kind: SemanticContainerKind,
        blocks: Vec<StagingM4Block>,
    },
}

impl StagingM4Block {
    pub const fn common(&self) -> &StagingM4BlockCommon {
        match self {
            Self::Paragraph { common, .. }
            | Self::Heading { common, .. }
            | Self::List { common, .. }
            | Self::Table { common, .. }
            | Self::Figure { common, .. }
            | Self::PageBreak { common }
            | Self::SemanticContainer { common, .. } => common,
        }
    }

    pub const fn node_id(&self) -> NodeId {
        self.common().node_id
    }

    pub const fn span(&self) -> SourceSpan {
        self.common().span
    }

    pub fn classes(&self) -> &[String] {
        &self.common().classes
    }

    pub const fn semantic_kind(&self) -> Option<SemanticContainerKind> {
        match self {
            Self::SemanticContainer { semantic_kind, .. } => Some(*semantic_kind),
            _ => None,
        }
    }

    pub fn direct_blocks(&self) -> &[StagingM4Block] {
        match self {
            Self::Figure { caption, .. }
            | Self::SemanticContainer {
                blocks: caption, ..
            } => caption,
            _ => &[],
        }
    }

    /// Profile-preflight view of ADR-0032 semantic emptiness. Structural
    /// wrappers and breaks are not content; authored inline text/references,
    /// an alternative-bearing replacement, or a nonempty owned subflow are.
    pub fn is_semantically_nonempty(&self) -> bool {
        match self {
            Self::Paragraph {
                has_authored_content,
                ..
            }
            | Self::Heading {
                has_authored_content,
                ..
            } => *has_authored_content,
            Self::List { items, .. } => items
                .iter()
                .flat_map(|item| &item.blocks)
                .any(Self::is_semantically_nonempty),
            Self::Table { head, body, .. } => head
                .iter()
                .chain(body)
                .flat_map(|row| &row.cells)
                .flat_map(|cell| &cell.blocks)
                .any(Self::is_semantically_nonempty),
            Self::Figure {
                has_nonempty_alternative,
                caption,
                ..
            } => *has_nonempty_alternative || caption.iter().any(Self::is_semantically_nonempty),
            Self::SemanticContainer { blocks, .. } => {
                blocks.iter().any(Self::is_semantically_nonempty)
            }
            Self::PageBreak { .. } => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4FootnoteDefinition {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub blocks: Vec<StagingM4Block>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4Document {
    pub node_id: NodeId,
    pub blocks: Vec<StagingM4Block>,
    pub footnotes: Vec<StagingM4FootnoteDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4FontFaceDeclaration {
    pub font_face_id: FontFaceId,
    pub family: String,
    pub uri: PortablePath,
    pub face_index: u32,
    pub expected_sha256: Option<[u8; 32]>,
    pub media: FontMediaDeclaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4ImageDeclaration {
    pub image_id: ImageResourceId,
    pub uri: PortablePath,
    pub expected_sha256: Option<[u8; 32]>,
    pub media: ImageMediaDeclaration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4ResourceCatalog {
    pub font_faces: Vec<StagingM4FontFaceDeclaration>,
    pub images: Vec<StagingM4ImageDeclaration>,
}
