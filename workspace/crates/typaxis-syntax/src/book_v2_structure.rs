//! Borrowed source structure projection. This is not a legacy accessibility
//! receipt or final reading order: footnote relocation and annotation relations
//! belong to the selected structure owner.
use super::{PreparedBookV2Navigation, PreparedBookV2TextFlow};
use typaxis_core::{NodeId, SourceSpan};
use typaxis_document::book_v2::BookV2LanguageNodeKind as K;
use typaxis_document_package::{book_v2::WireBookV2Block as Block, WireStagingM4Inline as Inline};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BookV2StructureSlot {
    Source,
    ListLabel,
    ListBody,
    TableHead,
    TableBody,
    Caption,
    FootnoteLink,
    FootnoteLabel,
    ReferenceLink,
    ReferenceLabel,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BookV2StructureKey {
    owner: NodeId,
    slot: BookV2StructureSlot,
}
impl BookV2StructureKey {
    pub const fn new(owner: NodeId, slot: BookV2StructureSlot) -> Self {
        Self { owner, slot }
    }
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn slot(self) -> BookV2StructureSlot {
        self.slot
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2StructureCell {
    pub table: NodeId,
    pub row: u32,
    pub column: u32,
    pub colspan: u16,
    pub rowspan: u16,
    pub header: bool,
}
/// All strings and spans originate in the exact prepared source/flow. Private
/// fields prevent a consumer from minting source descriptors.
#[derive(Clone, Copy, Debug)]
pub struct BookV2StructureSourceNode<'a> {
    key: BookV2StructureKey,
    parent: Option<BookV2StructureKey>,
    language: &'a str,
    span: Option<SourceSpan>,
    role: &'static str,
    semantic_kind: Option<&'static str>,
    alternative: Option<&'a str>,
    cell: Option<BookV2StructureCell>,
}
impl<'a> BookV2StructureSourceNode<'a> {
    pub const fn key(&self) -> BookV2StructureKey {
        self.key
    }
    pub const fn parent(&self) -> Option<BookV2StructureKey> {
        self.parent
    }
    pub const fn language(&self) -> &'a str {
        self.language
    }
    pub const fn source_span(&self) -> Option<SourceSpan> {
        self.span
    }
    pub const fn pdf_role(&self) -> &'static str {
        self.role
    }
    pub const fn semantic_kind(&self) -> Option<&'static str> {
        self.semantic_kind
    }
    pub const fn alternative(&self) -> Option<&'a str> {
        self.alternative
    }
    pub const fn table_cell(&self) -> Option<BookV2StructureCell> {
        self.cell
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2StructureSourceError {
    Identity,
    Depth,
    Nodes,
}
impl std::fmt::Display for BookV2StructureSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 structure source: {self:?}")
    }
}
impl std::error::Error for BookV2StructureSourceError {}
/// The caller charges traversal (including lookup work) and reserves every
/// retained node before accepting it. The walker itself allocates no collections.
pub trait BookV2StructureVisitor<'a> {
    type Error: From<BookV2StructureSourceError>;
    fn step(&mut self, work: usize) -> Result<(), Self::Error>;
    fn node(&mut self, node: BookV2StructureSourceNode<'a>) -> Result<(), Self::Error>;
}
pub fn visit_book_v2_structure<'a, V: BookV2StructureVisitor<'a>>(
    flow: &'a PreparedBookV2TextFlow<'_>,
    visitor: &mut V,
) -> Result<(), V::Error> {
    use BookV2StructureSlot as S;
    let mut walk = Walker {
        flow,
        nav: flow.navigation(),
        visitor,
        table_cursor: 0,
        nodes: flow.navigation().ast_node_count(),
    };
    let document = flow.body().body().wire().document();
    let root = walk.source(
        document.node_id,
        K::Document,
        None,
        "Document",
        None,
        None,
        None,
    )?;
    walk.blocks(&document.blocks, root, 2)?;
    for note in &document.footnotes {
        walk.depth(2)?;
        let node = walk.source(
            note.node_id,
            K::FootnoteDefinition,
            Some(root),
            "Note",
            None,
            None,
            None,
        )?;
        let link = walk.generated(node, S::FootnoteLink, "Link", 3)?;
        walk.generated(link, S::FootnoteLabel, "Lbl", 4)?;
        walk.blocks(&note.blocks, node, 3)?;
    }
    if walk.table_cursor != flow.tables().len() {
        return Err(BookV2StructureSourceError::Identity.into());
    }
    Ok(())
}
struct Walker<'a, 'f, 'v, V> {
    flow: &'a PreparedBookV2TextFlow<'f>,
    nav: &'a PreparedBookV2Navigation<'f>,
    visitor: &'v mut V,
    table_cursor: usize,
    nodes: u64,
}
impl<'a, V: BookV2StructureVisitor<'a>> Walker<'a, '_, '_, V> {
    fn charge_node(&mut self) -> Result<(), V::Error> {
        self.visitor.step(1)?;
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|n| *n <= self.flow.body().body().limits().get().max_ast_nodes)
            .ok_or(BookV2StructureSourceError::Nodes)?;
        Ok(())
    }
    fn depth(&mut self, depth: u32) -> Result<(), V::Error> {
        self.visitor.step(1)?;
        if depth > self.flow.body().body().limits().get().max_ast_nesting_depth {
            return Err(BookV2StructureSourceError::Depth.into());
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn source(
        &mut self,
        raw: u32,
        kind: K,
        parent: Option<BookV2StructureKey>,
        role: &'static str,
        semantic_kind: Option<&'static str>,
        alternative: Option<&'a str>,
        cell: Option<BookV2StructureCell>,
    ) -> Result<BookV2StructureKey, V::Error> {
        self.visitor
            .step(2 + self.nav.languages().len().checked_ilog2().unwrap_or(0) as usize)?;
        let record = self
            .nav
            .language(NodeId::new(raw))
            .ok_or(BookV2StructureSourceError::Identity)?;
        if record.kind() != kind || record.parent() != parent.map(|p| p.owner) {
            return Err(BookV2StructureSourceError::Identity.into());
        }
        let key = BookV2StructureKey::new(record.node_id(), BookV2StructureSlot::Source);
        self.visitor.node(BookV2StructureSourceNode {
            key,
            parent,
            language: record.effective_language(),
            span: record.source_span(),
            role,
            semantic_kind,
            alternative,
            cell,
        })?;
        Ok(key)
    }
    fn generated(
        &mut self,
        parent: BookV2StructureKey,
        slot: BookV2StructureSlot,
        role: &'static str,
        depth: u32,
    ) -> Result<BookV2StructureKey, V::Error> {
        self.depth(depth)?;
        self.visitor
            .step(2 + self.nav.languages().len().checked_ilog2().unwrap_or(0) as usize)?;
        let record = self
            .nav
            .language(parent.owner)
            .ok_or(BookV2StructureSourceError::Identity)?;
        let key = BookV2StructureKey::new(parent.owner, slot);
        self.charge_node()?;
        self.visitor.node(BookV2StructureSourceNode {
            key,
            parent: Some(parent),
            language: record.effective_language(),
            span: record.source_span(),
            role,
            semantic_kind: None,
            alternative: None,
            cell: None,
        })?;
        Ok(key)
    }
    fn blocks(
        &mut self,
        blocks: &'a [Block],
        parent: BookV2StructureKey,
        depth: u32,
    ) -> Result<(), V::Error> {
        use BookV2StructureSlot as S;
        for block in blocks {
            self.depth(depth)?;
            self.visitor.step(1)?;
            let raw = block.node_id();
            match block {
                Block::DescriptionList { items, .. } => {
                    let node =
                        self.source(raw, K::DescriptionList, Some(parent), "L", None, None, None)?;
                    for item in items {
                        self.depth(depth + 1)?;
                        let li = self.source(
                            item.node_id,
                            K::DescriptionItem,
                            Some(node),
                            "LI",
                            None,
                            None,
                            None,
                        )?;
                        self.depth(depth + 2)?;
                        let term = self.source(
                            item.term.node_id,
                            K::DescriptionTerm,
                            Some(li),
                            "Lbl",
                            None,
                            None,
                            None,
                        )?;
                        self.inlines(&item.term.children, term, depth + 3)?;
                        let body = self.generated(li, S::ListBody, "LBody", depth + 2)?;
                        self.blocks(&item.blocks, body, depth + 3)?;
                    }
                }
                Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                    let (kind, role) = if let Block::Heading { level, .. } = block {
                        (
                            K::Heading,
                            match level {
                                1 => "H1",
                                2 => "H2",
                                3 => "H3",
                                4 => "H4",
                                5 => "H5",
                                6 => "H6",
                                _ => return Err(BookV2StructureSourceError::Identity.into()),
                            },
                        )
                    } else {
                        (K::Paragraph, "P")
                    };
                    let node = self.source(raw, kind, Some(parent), role, None, None, None)?;
                    self.inlines(children, node, depth + 1)?;
                }
                Block::SemanticContainer {
                    semantic_kind,
                    blocks,
                    ..
                } => {
                    let kind = semantic_kind.as_str();
                    let role = if kind == "quote" {
                        "BlockQuote"
                    } else {
                        "Sect"
                    };
                    let node = self.source(
                        raw,
                        K::SemanticContainer,
                        Some(parent),
                        role,
                        Some(kind),
                        None,
                        None,
                    )?;
                    self.blocks(blocks, node, depth + 1)?;
                }
                Block::List { items, .. } => {
                    let node = self.source(raw, K::List, Some(parent), "L", None, None, None)?;
                    for item in items {
                        self.depth(depth + 1)?;
                        let li = self.source(
                            item.node_id,
                            K::ListItem,
                            Some(node),
                            "LI",
                            None,
                            None,
                            None,
                        )?;
                        self.generated(li, S::ListLabel, "Lbl", depth + 2)?;
                        let body = self.generated(li, S::ListBody, "LBody", depth + 2)?;
                        self.blocks(&item.blocks, body, depth + 3)?;
                    }
                }
                Block::Table {
                    caption,
                    head,
                    body,
                    ..
                } => {
                    let node =
                        self.source(raw, K::Table, Some(parent), "Table", None, None, None)?;
                    self.visitor.step(1)?;
                    let table = self
                        .flow
                        .tables()
                        .get(self.table_cursor)
                        .filter(|table| table.owner() == node.owner)
                        .ok_or(BookV2StructureSourceError::Identity)?;
                    self.table_cursor += 1;
                    if let Some(caption) = caption {
                        let cap = self.generated(node, S::Caption, "Caption", depth + 1)?;
                        self.blocks(caption, cap, depth + 2)?;
                    }
                    let mut row_index = 0;
                    for (rows, slot, role, header) in [
                        (head, S::TableHead, "THead", true),
                        (body, S::TableBody, "TBody", false),
                    ] {
                        let section = self.generated(node, slot, role, depth + 1)?;
                        for row in rows {
                            self.depth(depth + 2)?;
                            let prepared = table
                                .rows()
                                .get(row_index)
                                .ok_or(BookV2StructureSourceError::Identity)?;
                            if prepared.owner().get() != row.node_id
                                || prepared.cells().len() != row.cells.len()
                            {
                                return Err(BookV2StructureSourceError::Identity.into());
                            }
                            let tr = self.source(
                                row.node_id,
                                K::TableRow,
                                Some(section),
                                "TR",
                                None,
                                None,
                                None,
                            )?;
                            for (cell, prepared) in
                                row.cells.iter().zip(&table.cells()[prepared.cells()])
                            {
                                if prepared.owner().get() != cell.node_id {
                                    return Err(BookV2StructureSourceError::Identity.into());
                                }
                                self.depth(depth + 3)?;
                                let attributes = BookV2StructureCell {
                                    table: node.owner,
                                    row: prepared.row(),
                                    column: prepared.column(),
                                    colspan: prepared.colspan().get(),
                                    rowspan: prepared.rowspan().get(),
                                    header,
                                };
                                let td = self.source(
                                    cell.node_id,
                                    K::TableCell,
                                    Some(tr),
                                    if header { "TH" } else { "TD" },
                                    None,
                                    None,
                                    Some(attributes),
                                )?;
                                self.blocks(&cell.blocks, td, depth + 4)?;
                            }
                            row_index += 1;
                        }
                    }
                    if row_index != table.rows().len() {
                        return Err(BookV2StructureSourceError::Identity.into());
                    }
                }
                Block::Figure { alt, caption, .. } | Block::VectorFigure { alt, caption, .. } => {
                    let kind = if matches!(block, Block::Figure { .. }) {
                        K::Figure
                    } else {
                        K::VectorFigure
                    };
                    let node =
                        self.source(raw, kind, Some(parent), "Figure", None, Some(alt), None)?;
                    if !caption.is_empty() {
                        let cap = self.generated(node, S::Caption, "Caption", depth + 1)?;
                        self.blocks(caption, cap, depth + 2)?;
                    }
                }
                Block::DisplayMath { speech, .. } => {
                    self.source(
                        raw,
                        K::DisplayMath,
                        Some(parent),
                        "Formula",
                        None,
                        Some(speech),
                        None,
                    )?;
                }
                Block::MathVectorBlock { alt, .. } => {
                    let node = self.source(
                        raw,
                        K::MathVectorBlock,
                        Some(parent),
                        "Formula",
                        None,
                        Some(alt),
                        None,
                    )?;
                    // Equation numbers are source children, never part of Formula ActualText.
                    self.visitor.step(
                        2 + self.nav.languages().len().checked_ilog2().unwrap_or(0) as usize,
                    )?;
                    let language = self
                        .nav
                        .language(node.owner)
                        .ok_or(BookV2StructureSourceError::Identity)?;
                    let vector = language
                        .vector()
                        .ok_or(BookV2StructureSourceError::Identity)?;
                    if let Some(number) = vector.equation_number() {
                        self.depth(depth + 1)?;
                        self.visitor.node(BookV2StructureSourceNode {
                            key: BookV2StructureKey::new(number.node_id(), S::Source),
                            parent: Some(node),
                            language: language.effective_language(),
                            span: Some(number.span()),
                            role: "Span",
                            semantic_kind: None,
                            alternative: None,
                            cell: None,
                        })?;
                    }
                }
                Block::PageBreak { .. } => {}
            }
        }
        Ok(())
    }
    fn inlines(
        &mut self,
        inlines: &'a [Inline],
        parent: BookV2StructureKey,
        depth: u32,
    ) -> Result<(), V::Error> {
        use BookV2StructureSlot as S;
        for inline in inlines {
            self.depth(depth)?;
            self.visitor.step(1)?;
            let (kind, role, alt) = match inline {
                Inline::Text { .. } => (K::Text, "Span", None),
                Inline::InlineMath { speech, .. } => {
                    (K::InlineMath, "Formula", Some(speech.as_str()))
                }
                Inline::InlineVector { alt, .. } => (K::InlineVector, "Figure", Some(alt.as_str())),
                Inline::MathVector { alt, .. } => (K::MathVector, "Formula", Some(alt.as_str())),
                Inline::Emphasis { .. } => (K::Emphasis, "Em", None),
                Inline::Strong { .. } => (K::Strong, "Strong", None),
                Inline::Link { .. } => (K::Link, "Link", None),
                Inline::Reference { .. } => (K::Reference, "Reference", None),
                Inline::FootnoteReference { .. } => (K::FootnoteReference, "Reference", None),
                Inline::Anchor { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {
                    continue;
                }
            };
            let node = self.source(inline.node_id(), kind, Some(parent), role, None, alt, None)?;
            match inline {
                Inline::Emphasis { children, .. }
                | Inline::Strong { children, .. }
                | Inline::Link { children, .. } => self.inlines(children, node, depth + 1)?,
                Inline::Reference { .. } => {
                    let link = self.generated(node, S::ReferenceLink, "Link", depth + 1)?;
                    self.generated(link, S::ReferenceLabel, "Span", depth + 2)?;
                }
                Inline::FootnoteReference { .. } => {
                    let link = self.generated(node, S::FootnoteLink, "Link", depth + 1)?;
                    self.generated(link, S::FootnoteLabel, "Lbl", depth + 2)?;
                }
                _ => (),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "book_v2_structure_tests.rs"]
mod tests;
