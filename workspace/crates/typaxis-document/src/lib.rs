#![forbid(unsafe_code)]

use core::num::NonZeroU16;
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{
    AnchorId, FootnoteId, GeneratedBufferKey, GenerationKind, ImageResourceId, MasterId, NodeId,
    NonNegativeLength, PortablePath, PositiveLength, Rect, SafeUri, SourceSpan, TextSpan,
};

mod semantic_container;

pub use semantic_container::{
    FontMediaDeclaration, FontMediaType, ImageMediaDeclaration, ImageMediaType,
    SemanticContainerKind, StagingM4Block, StagingM4BlockCommon, StagingM4Document,
    StagingM4FigurePlacement, StagingM4FontFaceDeclaration, StagingM4FootnoteDefinition,
    StagingM4ImageDeclaration, StagingM4ListItem, StagingM4ResourceCatalog, StagingM4TableCell,
    StagingM4TableRow,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub node_id: NodeId,
    pub blocks: Vec<Block>,
    pub footnotes: Vec<FootnoteDefinition>,
}

/// Contract-1.3 page-region content.  This intentionally is not a `Document`:
/// the closed region grammar has no definitions, generated sites, containers,
/// links, or other body-only constructs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRegion {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub blocks: Vec<PageRegionBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PageRegionBlock {
    Paragraph {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        children: Vec<PageRegionInline>,
    },
    Heading {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        level: HeadingLevel,
        children: Vec<PageRegionInline>,
    },
}

impl PageRegionBlock {
    pub const fn node_id(&self) -> NodeId {
        match self {
            Self::Paragraph { node_id, .. } | Self::Heading { node_id, .. } => *node_id,
        }
    }

    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Paragraph { span, .. } | Self::Heading { span, .. } => *span,
        }
    }

    pub fn classes(&self) -> &[String] {
        match self {
            Self::Paragraph { classes, .. } | Self::Heading { classes, .. } => classes,
        }
    }

    pub fn children(&self) -> &[PageRegionInline] {
        match self {
            Self::Paragraph { children, .. } | Self::Heading { children, .. } => children,
        }
    }

    pub const fn style_block_name(&self) -> &'static str {
        match self {
            Self::Paragraph { .. } => "paragraph",
            Self::Heading { .. } => "heading",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PageRegionInline {
    Text {
        node_id: NodeId,
        span: SourceSpan,
        text_span: TextSpan,
    },
    SoftBreak {
        node_id: NodeId,
        span: SourceSpan,
    },
    HardBreak {
        node_id: NodeId,
        span: SourceSpan,
    },
}

impl PageRegionInline {
    pub const fn node_id(&self) -> NodeId {
        match self {
            Self::Text { node_id, .. }
            | Self::SoftBreak { node_id, .. }
            | Self::HardBreak { node_id, .. } => *node_id,
        }
    }

    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Text { span, .. }
            | Self::SoftBreak { span, .. }
            | Self::HardBreak { span, .. } => *span,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FigurePlacement {
    Block,
    Float,
}

/// Closed scheduler classes for the contract-1.3 non-wrapping float profile.
/// The declaration wire chooses only block versus float; this enum records the
/// deterministic owner-selected candidate/placement class.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FloatPlacementClass {
    Here,
    Top,
    Bottom,
    NextPage,
}

impl FloatPlacementClass {
    pub const ORDERED: [Self; 4] = [Self::Here, Self::Top, Self::Bottom, Self::NextPage];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Here => "here",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::NextPage => "next_page",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloatClearance {
    Zero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageProgression {
    LeftToRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageWritingMode {
    HorizontalTopToBottom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnFill {
    Sequential,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnBalance {
    None,
    LastPage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnLayout {
    pub count: NonZeroU16,
    pub gap: NonNegativeLength,
    pub fill: ColumnFill,
    pub balance: ColumnBalance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvancedPageMaster {
    pub master_id: MasterId,
    pub trim: Rect,
    pub header_content: Option<PageRegion>,
    pub footer_content: Option<PageRegion>,
    pub column_layout: Option<ColumnLayout>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvancedPageMasterSet {
    pub page_progression: PageProgression,
    pub writing_mode: PageWritingMode,
    pub masters: Vec<AdvancedPageMaster>,
}

impl AdvancedPageMasterSet {
    pub fn master(&self, master_id: &MasterId) -> Option<&AdvancedPageMaster> {
        self.masters
            .iter()
            .find(|master| &master.master_id == master_id)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTarget {
    Internal(AnchorId),
    Uri(SafeUri),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Inline {
    Text {
        node_id: NodeId,
        span: SourceSpan,
        text_span: TextSpan,
    },
    Emphasis {
        node_id: NodeId,
        span: SourceSpan,
        children: Vec<Inline>,
    },
    Strong {
        node_id: NodeId,
        span: SourceSpan,
        children: Vec<Inline>,
    },
    Link {
        node_id: NodeId,
        span: SourceSpan,
        target: LinkTarget,
        children: Vec<Inline>,
    },
    Anchor {
        node_id: NodeId,
        span: SourceSpan,
        anchor_id: AnchorId,
    },
    Reference {
        node_id: NodeId,
        span: SourceSpan,
        target: AnchorId,
        format: ReferenceFormat,
    },
    FootnoteReference {
        node_id: NodeId,
        span: SourceSpan,
        footnote_id: FootnoteId,
    },
    SoftBreak {
        node_id: NodeId,
        span: SourceSpan,
    },
    HardBreak {
        node_id: NodeId,
        span: SourceSpan,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceFormat {
    Text,
    Page,
    Number,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeadingLevel(u8);
impl HeadingLevel {
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value <= 6 {
            Some(Self(value))
        } else {
            None
        }
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColumnSizing {
    Fixed(PositiveLength),
    Fraction(NonZeroU16),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableColumn {
    pub sizing: ColumnSizing,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableCell {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub colspan: NonZeroU16,
    pub rowspan: NonZeroU16,
    pub blocks: Vec<Block>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRow {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub cells: Vec<TableCell>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListItem {
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub blocks: Vec<Block>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    Paragraph {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        children: Vec<Inline>,
    },
    Heading {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        level: HeadingLevel,
        anchor_id: Option<AnchorId>,
        children: Vec<Inline>,
    },
    List {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        ordered: bool,
        start: Option<u32>,
        items: Vec<ListItem>,
    },
    Table {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        columns: Vec<TableColumn>,
        head: Vec<TableRow>,
        body: Vec<TableRow>,
    },
    Figure {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
        image_id: ImageResourceId,
        alt: String,
        caption: Vec<Block>,
    },
    PageBreak {
        node_id: NodeId,
        span: SourceSpan,
        classes: Vec<String>,
    },
}
impl Block {
    pub fn classes(&self) -> &[String] {
        match self {
            Self::Paragraph { classes, .. }
            | Self::Heading { classes, .. }
            | Self::List { classes, .. }
            | Self::Table { classes, .. }
            | Self::Figure { classes, .. }
            | Self::PageBreak { classes, .. } => classes,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnoteDefinition {
    pub footnote_id: FootnoteId,
    pub node_id: NodeId,
    pub span: SourceSpan,
    pub blocks: Vec<Block>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontFaceDeclaration {
    pub font_face_id: typaxis_core::FontFaceId,
    pub family: String,
    pub uri: PortablePath,
    pub face_index: u32,
    pub expected_sha256: Option<[u8; 32]>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageDeclaration {
    pub image_id: ImageResourceId,
    pub uri: PortablePath,
    pub expected_sha256: Option<[u8; 32]>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCatalog {
    pub font_faces: Vec<FontFaceDeclaration>,
    pub images: Vec<ImageDeclaration>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GeneratedSiteTarget {
    Anchor(AnchorId),
    Footnote(FootnoteId),
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedSite {
    key: GeneratedBufferKey,
    target: GeneratedSiteTarget,
}
impl GeneratedSite {
    pub const fn key(&self) -> GeneratedBufferKey {
        self.key
    }
    pub const fn target(&self) -> &GeneratedSiteTarget {
        &self.target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentNodeIndexError {
    NonCanonicalNodeId,
    NonCanonicalFootnoteOrder,
    DuplicateAnchor,
    TooManyNodes,
    TooManyGeneratedSites,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DocumentNodeKind {
    Document,
    Paragraph,
    Heading,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    Figure,
    PageBreak,
    Text,
    InlineContainer,
    Link,
    Anchor,
    Reference,
    FootnoteReference,
    SoftBreak,
    HardBreak,
    FootnoteDefinition,
}

/// Canonical node and generated-site registry derived exclusively from the
/// document's typed preorder. Generated overlays must name one of these sites.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedDocumentNodeIndex {
    nodes: BTreeSet<NodeId>,
    node_kinds: BTreeMap<NodeId, DocumentNodeKind>,
    node_paths: BTreeMap<NodeId, Vec<u32>>,
    generated_sites: BTreeMap<GeneratedBufferKey, GeneratedSite>,
    anchors: BTreeMap<AnchorId, NodeId>,
    footnote_reference_targets: BTreeSet<FootnoteId>,
}
impl ValidatedDocumentNodeIndex {
    pub fn empty_document() -> Self {
        Self {
            nodes: BTreeSet::from([NodeId::new(0)]),
            node_kinds: BTreeMap::from([(NodeId::new(0), DocumentNodeKind::Document)]),
            node_paths: BTreeMap::from([(NodeId::new(0), Vec::new())]),
            generated_sites: BTreeMap::new(),
            anchors: BTreeMap::new(),
            footnote_reference_targets: BTreeSet::new(),
        }
    }
    pub fn new(document: &Document) -> Result<Self, DocumentNodeIndexError> {
        let mut builder = DocumentNodeIndexBuilder {
            next_node_id: 0,
            nodes: BTreeSet::new(),
            node_kinds: BTreeMap::new(),
            node_paths: BTreeMap::new(),
            generated_sites: BTreeMap::new(),
            anchors: BTreeMap::new(),
            next_generated_ordinal: BTreeMap::new(),
            footnote_reference_targets: BTreeSet::new(),
        };
        builder.node(document.node_id, DocumentNodeKind::Document, Vec::new())?;
        for (index, block) in document.blocks.iter().enumerate() {
            builder.block(block, child_path(&[], 0, index)?)?;
        }
        let mut previous_footnote: Option<&FootnoteId> = None;
        for (index, footnote) in document.footnotes.iter().enumerate() {
            if previous_footnote.is_some_and(|previous| previous >= &footnote.footnote_id) {
                return Err(DocumentNodeIndexError::NonCanonicalFootnoteOrder);
            }
            let footnote_path = child_path(&[], 1, index)?;
            builder.node(
                footnote.node_id,
                DocumentNodeKind::FootnoteDefinition,
                footnote_path.clone(),
            )?;
            builder.site(
                footnote.node_id,
                GenerationKind::FootnoteMarker,
                GeneratedSiteTarget::None,
            )?;
            for (block_index, block) in footnote.blocks.iter().enumerate() {
                builder.block(block, child_path(&footnote_path, 0, block_index)?)?;
            }
            previous_footnote = Some(&footnote.footnote_id);
        }
        Ok(Self {
            nodes: builder.nodes,
            node_kinds: builder.node_kinds,
            node_paths: builder.node_paths,
            generated_sites: builder.generated_sites,
            anchors: builder.anchors,
            footnote_reference_targets: builder.footnote_reference_targets,
        })
    }
    pub fn contains_node(&self, node_id: NodeId) -> bool {
        self.nodes.contains(&node_id)
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn node_kind(&self, node_id: NodeId) -> Option<DocumentNodeKind> {
        self.node_kinds.get(&node_id).copied()
    }
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = (NodeId, DocumentNodeKind)> + '_ {
        self.node_kinds.iter().map(|(id, kind)| (*id, *kind))
    }
    /// Typed-child preorder path. Each pair is `(field_tag, child_index)`;
    /// field tags are fixed by the owning document variant, never map order.
    pub fn node_path(&self, node_id: NodeId) -> Option<&[u32]> {
        self.node_paths.get(&node_id).map(Vec::as_slice)
    }
    pub fn generated_site(&self, key: GeneratedBufferKey) -> Option<&GeneratedSite> {
        self.generated_sites.get(&key)
    }
    pub fn generated_sites(&self) -> impl ExactSizeIterator<Item = &GeneratedSite> {
        self.generated_sites.values()
    }
    pub fn anchor_owner(&self, anchor_id: &AnchorId) -> Option<NodeId> {
        self.anchors.get(anchor_id).copied()
    }
    pub fn anchors(&self) -> impl ExactSizeIterator<Item = (&AnchorId, NodeId)> {
        self.anchors.iter().map(|(id, owner)| (id, *owner))
    }
    pub const fn footnote_reference_targets(&self) -> &BTreeSet<FootnoteId> {
        &self.footnote_reference_targets
    }
}

struct DocumentNodeIndexBuilder {
    next_node_id: u32,
    nodes: BTreeSet<NodeId>,
    node_kinds: BTreeMap<NodeId, DocumentNodeKind>,
    node_paths: BTreeMap<NodeId, Vec<u32>>,
    generated_sites: BTreeMap<GeneratedBufferKey, GeneratedSite>,
    anchors: BTreeMap<AnchorId, NodeId>,
    next_generated_ordinal: BTreeMap<(NodeId, GenerationKind), u32>,
    footnote_reference_targets: BTreeSet<FootnoteId>,
}
impl DocumentNodeIndexBuilder {
    fn node(
        &mut self,
        node_id: NodeId,
        kind: DocumentNodeKind,
        path: Vec<u32>,
    ) -> Result<(), DocumentNodeIndexError> {
        if node_id.get() != self.next_node_id || !self.nodes.insert(node_id) {
            return Err(DocumentNodeIndexError::NonCanonicalNodeId);
        }
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .ok_or(DocumentNodeIndexError::TooManyNodes)?;
        self.node_kinds.insert(node_id, kind);
        self.node_paths.insert(node_id, path);
        Ok(())
    }
    fn site(
        &mut self,
        owner: NodeId,
        generation_kind: GenerationKind,
        target: GeneratedSiteTarget,
    ) -> Result<(), DocumentNodeIndexError> {
        let ordinal = self
            .next_generated_ordinal
            .entry((owner, generation_kind))
            .or_insert(0);
        let key = GeneratedBufferKey::new(owner, generation_kind, *ordinal);
        *ordinal = ordinal
            .checked_add(1)
            .ok_or(DocumentNodeIndexError::TooManyGeneratedSites)?;
        self.generated_sites
            .insert(key, GeneratedSite { key, target });
        Ok(())
    }
    fn anchor(
        &mut self,
        anchor_id: &AnchorId,
        owner: NodeId,
    ) -> Result<(), DocumentNodeIndexError> {
        if self.anchors.insert(anchor_id.clone(), owner).is_some() {
            return Err(DocumentNodeIndexError::DuplicateAnchor);
        }
        Ok(())
    }
    fn block(&mut self, block: &Block, path: Vec<u32>) -> Result<(), DocumentNodeIndexError> {
        match block {
            Block::Paragraph {
                node_id, children, ..
            } => {
                self.node(*node_id, DocumentNodeKind::Paragraph, path.clone())?;
                self.inlines(children, &path)
            }
            Block::Heading {
                node_id,
                anchor_id,
                children,
                ..
            } => {
                self.node(*node_id, DocumentNodeKind::Heading, path.clone())?;
                if let Some(anchor_id) = anchor_id {
                    self.anchor(anchor_id, *node_id)?;
                }
                self.inlines(children, &path)
            }
            Block::List { node_id, items, .. } => {
                self.node(*node_id, DocumentNodeKind::List, path.clone())?;
                for (item_index, item) in items.iter().enumerate() {
                    let item_path = child_path(&path, 0, item_index)?;
                    self.node(item.node_id, DocumentNodeKind::ListItem, item_path.clone())?;
                    self.site(
                        item.node_id,
                        GenerationKind::ListMarker,
                        GeneratedSiteTarget::None,
                    )?;
                    for (block_index, block) in item.blocks.iter().enumerate() {
                        self.block(block, child_path(&item_path, 0, block_index)?)?;
                    }
                }
                Ok(())
            }
            Block::Table {
                node_id,
                head,
                body,
                ..
            } => {
                self.node(*node_id, DocumentNodeKind::Table, path.clone())?;
                for (row_index, row) in head.iter().enumerate() {
                    self.table_row(row, child_path(&path, 0, row_index)?)?;
                }
                for (row_index, row) in body.iter().enumerate() {
                    self.table_row(row, child_path(&path, 1, row_index)?)?;
                }
                Ok(())
            }
            Block::Figure {
                node_id, caption, ..
            } => {
                self.node(*node_id, DocumentNodeKind::Figure, path.clone())?;
                for (block_index, block) in caption.iter().enumerate() {
                    self.block(block, child_path(&path, 0, block_index)?)?;
                }
                Ok(())
            }
            Block::PageBreak { node_id, .. } => {
                self.node(*node_id, DocumentNodeKind::PageBreak, path)
            }
        }
    }
    fn table_row(&mut self, row: &TableRow, path: Vec<u32>) -> Result<(), DocumentNodeIndexError> {
        self.node(row.node_id, DocumentNodeKind::TableRow, path.clone())?;
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let cell_path = child_path(&path, 0, cell_index)?;
            self.node(cell.node_id, DocumentNodeKind::TableCell, cell_path.clone())?;
            for (block_index, block) in cell.blocks.iter().enumerate() {
                self.block(block, child_path(&cell_path, 0, block_index)?)?;
            }
        }
        Ok(())
    }
    fn inlines(
        &mut self,
        inlines: &[Inline],
        parent_path: &[u32],
    ) -> Result<(), DocumentNodeIndexError> {
        for (index, inline) in inlines.iter().enumerate() {
            let path = child_path(parent_path, 0, index)?;
            match inline {
                Inline::Text { node_id, .. } => {
                    self.node(*node_id, DocumentNodeKind::Text, path)?
                }
                Inline::Anchor {
                    node_id, anchor_id, ..
                } => {
                    self.node(*node_id, DocumentNodeKind::Anchor, path)?;
                    self.anchor(anchor_id, *node_id)?;
                }
                Inline::HardBreak { node_id, .. } => {
                    self.node(*node_id, DocumentNodeKind::HardBreak, path)?;
                    self.site(
                        *node_id,
                        GenerationKind::Discretionary,
                        GeneratedSiteTarget::None,
                    )?;
                }
                Inline::Emphasis {
                    node_id, children, ..
                }
                | Inline::Strong {
                    node_id, children, ..
                } => {
                    self.node(*node_id, DocumentNodeKind::InlineContainer, path.clone())?;
                    self.inlines(children, &path)?;
                }
                Inline::Link {
                    node_id, children, ..
                } => {
                    self.node(*node_id, DocumentNodeKind::Link, path.clone())?;
                    self.inlines(children, &path)?;
                }
                Inline::Reference {
                    node_id,
                    target,
                    format,
                    ..
                } => {
                    self.node(*node_id, DocumentNodeKind::Reference, path)?;
                    let kind = match format {
                        ReferenceFormat::Page => GenerationKind::PageReference,
                        ReferenceFormat::Text | ReferenceFormat::Number => GenerationKind::Counter,
                    };
                    self.site(*node_id, kind, GeneratedSiteTarget::Anchor(target.clone()))?;
                }
                Inline::FootnoteReference {
                    node_id,
                    footnote_id,
                    ..
                } => {
                    self.node(*node_id, DocumentNodeKind::FootnoteReference, path)?;
                    self.footnote_reference_targets.insert(footnote_id.clone());
                    self.site(
                        *node_id,
                        GenerationKind::FootnoteMarker,
                        GeneratedSiteTarget::Footnote(footnote_id.clone()),
                    )?;
                }
                Inline::SoftBreak { node_id, .. } => {
                    self.node(*node_id, DocumentNodeKind::SoftBreak, path)?;
                    self.site(
                        *node_id,
                        GenerationKind::Discretionary,
                        GeneratedSiteTarget::None,
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn child_path(
    parent: &[u32],
    field_tag: u32,
    child_index: usize,
) -> Result<Vec<u32>, DocumentNodeIndexError> {
    let child_index =
        u32::try_from(child_index).map_err(|_| DocumentNodeIndexError::TooManyNodes)?;
    let capacity = parent
        .len()
        .checked_add(2)
        .ok_or(DocumentNodeIndexError::TooManyNodes)?;
    let mut path = Vec::new();
    path.try_reserve_exact(capacity)
        .map_err(|_| DocumentNodeIndexError::TooManyNodes)?;
    path.extend_from_slice(parent);
    path.push(field_tag);
    path.push(child_index);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn heading_level_is_limited() {
        assert!(HeadingLevel::new(0).is_none());
        assert!(HeadingLevel::new(6).is_some());
        assert!(HeadingLevel::new(7).is_none());
    }
}
