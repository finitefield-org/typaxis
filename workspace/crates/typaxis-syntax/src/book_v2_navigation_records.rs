//! Borrowed scalar census, before any navigation graph allocation. Reservations
//! bound logical record collections; source text, vector capacity, tree nodes
//! and Unicode canonicalization temporaries still require separate byte bounds.
use super::{BookNavigationSyntaxError, NodeId, StyledBookV2Body};
use typaxis_document_package::book_v2::WireBookV2Block as Block;
use typaxis_document_package::{
    WirePageRegionBlock, WirePageRegionInline, WireStagingM4Inline as Inline,
    WireStagingM4LinkTarget,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookV2NavigationPreparationError {
    /// No graph allocation or source clone is needed to produce this error.
    RecordLimit {
        owner: NodeId,
    },
    Syntax(BookNavigationSyntaxError),
}
impl std::fmt::Display for BookV2NavigationPreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordLimit { owner } => {
                write!(f, "navigation record limit at node {}", owner.get())
            }
            Self::Syntax(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for BookV2NavigationPreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Syntax(error) => Some(error),
            Self::RecordLimit { .. } => None,
        }
    }
}
type Error = BookV2NavigationPreparationError;

struct Budget<'a> {
    maximum: u64,
    observed: &'a mut u64,
}
impl Budget<'_> {
    fn reserve(&mut self, owner: u32, count: u64) -> Result<(), Error> {
        let next = self
            .observed
            .checked_add(count)
            .filter(|next| *next <= self.maximum)
            .ok_or(Error::RecordLimit {
                owner: NodeId::new(owner),
            })?;
        *self.observed = next;
        Ok(())
    }
    fn language(&mut self, owner: u32, explicit: bool) -> Result<(), Error> {
        // Source site, a full-length bound for stable-sort scratch, and prepared
        // record. Each explicit spelling can add one distinct intern-pool entry.
        self.reserve(owner, 3)?;
        if explicit {
            self.reserve(owner, 1)?;
        }
        Ok(())
    }
    fn anchor(&mut self, owner: u32) -> Result<(), Error> {
        // Raw anchor map and retained sorted anchor array.
        self.reserve(owner, 2)
    }
    fn inlines(&mut self, values: &[Inline]) -> Result<(), Error> {
        for value in values {
            let id = value.node_id();
            match value {
                Inline::Anchor { .. } => self.anchor(id)?,
                Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {}
                Inline::Text { .. }
                | Inline::InlineMath { .. }
                | Inline::InlineVector { .. }
                | Inline::MathVector { .. }
                | Inline::Reference { .. }
                | Inline::FootnoteReference { .. }
                | Inline::Emphasis { .. }
                | Inline::Strong { .. }
                | Inline::Link { .. } => {
                    self.language(id, value.language().is_some())?;
                }
            }
            match value {
                Inline::Reference { .. } | Inline::FootnoteReference { .. } => {
                    // Raw target/path collection and prepared reference array.
                    self.reserve(id, 2)?;
                }
                Inline::Link {
                    target: WireStagingM4LinkTarget::Internal { .. },
                    ..
                } => {
                    // Raw links, stable-sort scratch and prepared links.
                    self.reserve(id, 3)?;
                }
                _ => {}
            }
            match value {
                Inline::Emphasis { children, .. }
                | Inline::Strong { children, .. }
                | Inline::Link { children, .. } => self.inlines(children)?,
                _ => {}
            }
        }
        Ok(())
    }
    fn blocks(&mut self, values: &[Block]) -> Result<(), Error> {
        for value in values {
            let id = value.node_id();
            let language = match value {
                Block::Paragraph { language, .. }
                | Block::Heading { language, .. }
                | Block::SemanticContainer { language, .. }
                | Block::List { language, .. }
                | Block::DescriptionList { language, .. }
                | Block::Table { language, .. }
                | Block::Figure { language, .. }
                | Block::VectorFigure { language, .. }
                | Block::DisplayMath { language, .. }
                | Block::MathVectorBlock { language, .. } => Some(language),
                Block::PageBreak { .. } => None,
            };
            if let Some(language) = language {
                self.language(id, language.is_some())?;
            }
            match value {
                Block::Heading { anchor_id, .. } | Block::SemanticContainer { anchor_id, .. } => {
                    self.reserve(id, 1)?; // Outline-owner registry, even without an outline entry.
                    if anchor_id.is_some() {
                        self.anchor(id)?;
                    }
                }
                Block::MathVectorBlock {
                    equation_number: Some(_),
                    ..
                } => {
                    self.reserve(id, 1)?; // Child language record, separate from ordinary sites.
                }
                _ => {}
            }
            match value {
                Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                    self.inlines(children)?;
                }
                Block::SemanticContainer { blocks, .. } => self.blocks(blocks)?,
                Block::Figure { caption, .. } | Block::VectorFigure { caption, .. } => {
                    self.blocks(caption)?;
                }
                Block::List { items, .. } => {
                    for item in items {
                        self.language(item.node_id, item.language.is_some())?;
                        self.blocks(&item.blocks)?;
                    }
                }
                Block::DescriptionList { items, .. } => {
                    for item in items {
                        self.language(item.node_id, item.language.is_some())?;
                        self.language(item.term.node_id, item.term.language.is_some())?;
                        self.inlines(&item.term.children)?;
                        self.blocks(&item.blocks)?;
                    }
                }
                Block::Table {
                    caption,
                    head,
                    body,
                    ..
                } => {
                    if let Some(caption) = caption {
                        self.blocks(caption)?;
                    }
                    for row in head.iter().chain(body) {
                        self.language(row.node_id, row.language.is_some())?;
                        for cell in &row.cells {
                            self.language(cell.node_id, cell.language.is_some())?;
                            self.blocks(&cell.blocks)?;
                        }
                    }
                }
                Block::PageBreak { .. }
                | Block::DisplayMath { .. }
                | Block::MathVectorBlock { .. } => {}
            }
        }
        Ok(())
    }
}

pub(super) fn reserve(
    body: &StyledBookV2Body,
    maximum: u64,
    observed: &mut u64,
) -> Result<(), Error> {
    let prepared = body.body();
    let wire = prepared.wire();
    let document = wire.document();
    let mut budget = Budget {
        maximum: maximum.min(prepared.limits().get().max_fragments),
        observed,
    };
    budget.reserve(document.node_id, 1)?; // Navigation owner.
    budget.reserve(document.node_id, 1)?; // Metadata carrier.
    for _ in &wire.metadata().keywords {
        budget.reserve(document.node_id, 1)?;
    }
    budget.language(document.node_id, true)?;
    budget.blocks(&document.blocks)?;
    for note in &document.footnotes {
        budget.language(note.node_id, note.language.is_some())?;
        budget.reserve(note.node_id, 1)?; // Borrowed definition registry used by references.
        budget.blocks(&note.blocks)?;
    }
    // Language validation includes unselected masters, but region wrappers and
    // break controls are deliberately not ordinary language records.
    for master in &wire.advanced_page_masters().masters {
        for region in [
            master.header_content.as_ref(),
            master.footer_content.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for block in &region.blocks {
                let (id, children) = match block {
                    WirePageRegionBlock::Paragraph {
                        node_id, children, ..
                    }
                    | WirePageRegionBlock::Heading {
                        node_id, children, ..
                    } => (*node_id, children),
                };
                budget.language(id, false)?;
                for child in children {
                    if let WirePageRegionInline::Text { node_id, .. } = child {
                        budget.language(*node_id, false)?;
                    }
                }
            }
        }
    }
    for binding in document.number_bindings.iter().flatten() {
        // At most three wanted slots and three found records per binding,
        // one seen entry, one prepared binding, and a possible virtual anchor
        // in both raw and retained registries. Shared/existing anchors can make
        // the actual count smaller; no allocation-time deduplication is needed
        // to prove this upper bound.
        budget.reserve(binding.owner_node_id, 10)?;
    }
    for entry in &wire.outline().entries {
        // Output, validation stack, source set and destination set.
        budget.reserve(entry.source_node_id, 4)?;
    }
    Ok(())
}
