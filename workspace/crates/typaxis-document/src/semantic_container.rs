use core::num::NonZeroU16;
use typaxis_core::{
    FontFaceId, ImageResourceId, Length, NodeId, NonNegativeLength, PortablePath, PositiveLength,
    SourceSpan, TextSpan,
};

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
    JpegBaseline,
    SvgSafe1,
    SvgSafe2,
}

impl ImageMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::JpegBaseline => "jpeg-baseline",
            Self::SvgSafe1 => "svg-safe-1",
            Self::SvgSafe2 => "svg-safe-2",
        }
    }
}

/// Producer assertion retained byte-for-byte for `svg-safe-2` declarations.
/// Validation and parser selection remain owned by later sealed stages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorProvenance {
    pub engine_id: String,
    pub engine_version: String,
    pub rules_version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorViewport {
    pub width: PositiveLength,
    pub height: PositiveLength,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorMetrics {
    pub advance: PositiveLength,
    pub ascent: PositiveLength,
    pub baseline: NonNegativeLength,
    pub descent: NonNegativeLength,
    pub origin_x: Length,
    pub viewport: PrecomposedVectorViewport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorSpacing {
    pub before: NonNegativeLength,
    pub after: NonNegativeLength,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorSourceTex {
    pub text_span: TextSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorEquationNumber {
    pub minimum_gap: PositiveLength,
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub text_span: TextSpan,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingM4InlineVectorKind {
    InlineVector,
    MathVector,
}

impl StagingM4InlineVectorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
        }
    }
}

/// Lossless private contract-1.4 inline-vector record. Nested inline wrappers
/// are represented by the global dense preorder and the owning block ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4InlineVector {
    pub node_id: NodeId,
    pub owner_node_id: NodeId,
    pub kind: StagingM4InlineVectorKind,
    pub span: SourceSpan,
    pub image_id: ImageResourceId,
    pub metrics: PrecomposedVectorMetrics,
    pub spacing: PrecomposedVectorSpacing,
    pub source_tex: Option<PrecomposedVectorSourceTex>,
    pub alternative: String,
    pub actual_text: Option<String>,
    pub language: Option<String>,
}

/// Closed TrueType-outline containers initially supported by 1.4 staging.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FontMediaType {
    SfntTrueTypeGlyf,
    TtcTrueTypeGlyf,
    SfntCff1,
}

impl FontMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SfntTrueTypeGlyf => "sfnt-truetype-glyf",
            Self::TtcTrueTypeGlyf => "ttc-truetype-glyf",
            Self::SfntCff1 => "sfnt-cff1",
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

/// The owning node, never delimiter syntax, closes the math layout mode.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingM4MathKind {
    Inline,
    Display,
}

impl StagingM4MathKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline_math",
            Self::Display => "display_math",
        }
    }
}

/// Lossless private contract-1.4 math domain. Syntax proves the TextMap
/// ownership and replaces none of these authored bytes with formatted text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4MathNode {
    pub node_id: NodeId,
    pub owner_node_id: NodeId,
    pub kind: StagingM4MathKind,
    pub span: SourceSpan,
    pub text_span: TextSpan,
    pub language: String,
    pub version: String,
    pub source: String,
    pub speech: String,
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
        inline_vectors: Vec<StagingM4InlineVector>,
    },
    Heading {
        common: StagingM4BlockCommon,
        has_authored_content: bool,
        inline_vectors: Vec<StagingM4InlineVector>,
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
        image_id: ImageResourceId,
        placement: StagingM4FigurePlacement,
        alternative: String,
        has_nonempty_alternative: bool,
        caption: Vec<StagingM4Block>,
    },
    PageBreak {
        common: StagingM4BlockCommon,
    },
    DisplayMath {
        common: StagingM4BlockCommon,
    },
    VectorFigure {
        common: StagingM4BlockCommon,
        image_id: ImageResourceId,
        viewport: PrecomposedVectorViewport,
        alternative: String,
        caption: Vec<StagingM4Block>,
        language: Option<String>,
    },
    MathVectorBlock {
        common: StagingM4BlockCommon,
        image_id: ImageResourceId,
        metrics: PrecomposedVectorMetrics,
        source_tex: PrecomposedVectorSourceTex,
        alternative: String,
        actual_text: Option<String>,
        equation_number: Option<PrecomposedVectorEquationNumber>,
        language: Option<String>,
    },
    SemanticContainer {
        common: StagingM4BlockCommon,
        semantic_kind: SemanticContainerKind,
        blocks: Vec<StagingM4Block>,
    },
}

/// Closed contract-1.4 Figure placement. Retaining the typed value and the
/// use-specific alternative keeps vector admission facts separate from layout
/// and accessibility facts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingM4FigurePlacement {
    Block,
    Float,
}

impl StagingM4FigurePlacement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Float => "float",
        }
    }
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
            | Self::DisplayMath { common }
            | Self::VectorFigure { common, .. }
            | Self::MathVectorBlock { common, .. }
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
            Self::Paragraph { .. }
            | Self::Heading { .. }
            | Self::List { .. }
            | Self::Table { .. }
            | Self::Figure { .. }
            | Self::PageBreak { .. }
            | Self::DisplayMath { .. }
            | Self::VectorFigure { .. }
            | Self::MathVectorBlock { .. } => None,
        }
    }

    pub fn direct_blocks(&self) -> &[StagingM4Block] {
        match self {
            Self::Figure { caption, .. }
            | Self::VectorFigure { caption, .. }
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
            Self::VectorFigure { .. } | Self::MathVectorBlock { .. } => true,
            Self::SemanticContainer { blocks, .. } => {
                blocks.iter().any(Self::is_semantically_nonempty)
            }
            Self::DisplayMath { .. } => true,
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
    pub vector_provenance: Option<VectorProvenance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4ResourceCatalog {
    pub font_faces: Vec<StagingM4FontFaceDeclaration>,
    pub images: Vec<StagingM4ImageDeclaration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{SourceId, TextBufferId, Utf8ByteOffset};

    fn length(raw: i64) -> Length {
        Length::from_raw(raw).unwrap()
    }

    fn positive(raw: i64) -> PositiveLength {
        PositiveLength::new(length(raw)).unwrap()
    }

    fn nonnegative(raw: i64) -> NonNegativeLength {
        NonNegativeLength::new(length(raw)).unwrap()
    }

    fn span(start: u32, end: u32) -> SourceSpan {
        SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(start),
            Utf8ByteOffset::new(end),
        )
        .unwrap()
    }

    fn text_span(start: u32, end: u32) -> TextSpan {
        TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(start),
            Utf8ByteOffset::new(end),
        )
        .unwrap()
    }

    fn metrics() -> PrecomposedVectorMetrics {
        PrecomposedVectorMetrics {
            advance: positive(20),
            ascent: positive(10),
            baseline: nonnegative(8),
            descent: nonnegative(3),
            origin_x: length(-1),
            viewport: PrecomposedVectorViewport {
                width: positive(19),
                height: positive(11),
            },
        }
    }

    fn common(node_id: u32, start: u32, end: u32) -> StagingM4BlockCommon {
        StagingM4BlockCommon {
            node_id: NodeId::new(node_id),
            span: span(start, end),
            classes: vec!["vector".to_owned()],
        }
    }

    #[test]
    fn precomposed_vector_domain_retains_all_four_kinds_and_authored_content() {
        let inline_vector = StagingM4InlineVector {
            node_id: NodeId::new(2),
            owner_node_id: NodeId::new(1),
            kind: StagingM4InlineVectorKind::InlineVector,
            span: span(0, 1),
            image_id: ImageResourceId::new(0),
            metrics: metrics(),
            spacing: PrecomposedVectorSpacing {
                before: nonnegative(1),
                after: nonnegative(2),
            },
            source_tex: None,
            alternative: "diagram".to_owned(),
            actual_text: Some("diagram text".to_owned()),
            language: Some("en".to_owned()),
        };
        let math_vector = StagingM4InlineVector {
            node_id: NodeId::new(3),
            owner_node_id: NodeId::new(1),
            kind: StagingM4InlineVectorKind::MathVector,
            span: span(1, 4),
            image_id: ImageResourceId::new(0),
            metrics: metrics(),
            spacing: PrecomposedVectorSpacing {
                before: nonnegative(1),
                after: nonnegative(1),
            },
            source_tex: Some(PrecomposedVectorSourceTex {
                text_span: text_span(1, 4),
            }),
            alternative: "x plus y".to_owned(),
            actual_text: None,
            language: None,
        };
        let paragraph = StagingM4Block::Paragraph {
            common: common(1, 0, 4),
            has_authored_content: true,
            inline_vectors: vec![inline_vector.clone(), math_vector.clone()],
        };
        let vector_figure = StagingM4Block::VectorFigure {
            common: common(4, 4, 5),
            image_id: ImageResourceId::new(0),
            viewport: metrics().viewport,
            alternative: "figure".to_owned(),
            caption: vec![],
            language: None,
        };
        let math_block = StagingM4Block::MathVectorBlock {
            common: common(5, 5, 11),
            image_id: ImageResourceId::new(0),
            metrics: metrics(),
            source_tex: PrecomposedVectorSourceTex {
                text_span: text_span(5, 8),
            },
            alternative: "equation".to_owned(),
            actual_text: None,
            equation_number: Some(PrecomposedVectorEquationNumber {
                minimum_gap: positive(1),
                node_id: NodeId::new(6),
                span: span(8, 11),
                text_span: text_span(8, 11),
            }),
            language: Some("ja".to_owned()),
        };

        assert!(paragraph.is_semantically_nonempty());
        assert!(vector_figure.is_semantically_nonempty());
        assert!(math_block.is_semantically_nonempty());
        let StagingM4Block::Paragraph { inline_vectors, .. } = paragraph else {
            unreachable!();
        };
        assert_eq!(inline_vectors, vec![inline_vector, math_vector]);
        assert_eq!(vector_figure.direct_blocks(), &[]);
        assert_eq!(math_block.node_id(), NodeId::new(5));
        assert_eq!(ImageMediaType::SvgSafe2.as_str(), "svg-safe-2");
        assert_eq!(
            crate::StagingLanguageNodeKind::MathVectorBlock.as_str(),
            "math_vector_block"
        );

        let resource = StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(0),
            uri: PortablePath::new("math/vector.svg").unwrap(),
            expected_sha256: Some([7; 32]),
            media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
            vector_provenance: Some(VectorProvenance {
                engine_id: "vmb.texToSvg".to_owned(),
                engine_version: "2026.09.0".to_owned(),
                rules_version: "vmb.math-safe-svg/1".to_owned(),
            }),
        };
        assert_eq!(
            resource
                .vector_provenance
                .as_ref()
                .map(|value| value.rules_version.as_str()),
            Some("vmb.math-safe-svg/1")
        );
    }
}
