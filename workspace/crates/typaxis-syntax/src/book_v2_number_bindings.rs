use super::*;
use typaxis_document_package::book_v2::{WireBookV2Block as Block, WireBookV2Document};
use typaxis_document_package::{WireStagingM4Inline as Inline, WireStagingTextSpan};

#[derive(Debug)]
pub struct PreparedBookV2NumberBinding<'a> {
    anchor: AnchorId,
    owner: NodeId,
    label_owner: NodeId,
    text_span: WireStagingTextSpan,
    label: &'a str,
}
impl<'a> PreparedBookV2NumberBinding<'a> {
    pub fn anchor_id(&self) -> &AnchorId {
        &self.anchor
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn label_owner(&self) -> NodeId {
        self.label_owner
    }
    pub const fn text_span(&self) -> WireStagingTextSpan {
        self.text_span
    }
    pub const fn label(&self) -> &'a str {
        self.label
    }
}

#[derive(Clone, Copy)]
struct NumberNode {
    id: u32,
    last: u32,
    can_own: bool,
    text: Option<WireStagingTextSpan>,
}

// Dense preorder and parent/source containment have already been checked by
// the exact body owner. Subtree intervals include inline anchors and equation
// number leaves, which are deliberately not ordinary language-site records.
struct Index<'a> {
    wanted: &'a [u32],
    found: Vec<NumberNode>,
}
impl Index<'_> {
    fn add(
        &mut self,
        id: u32,
        last: u32,
        can_own: bool,
        text: Option<WireStagingTextSpan>,
    ) -> Result<u32, BookNavigationSyntaxError> {
        if self.wanted.binary_search(&id).is_ok() {
            self.found
                .try_reserve(1)
                .map_err(|_| allocation("/document/number_bindings"))?;
            self.found.push(NumberNode {
                id,
                last,
                can_own,
                text,
            });
        }
        Ok(last)
    }
    fn inlines(
        &mut self,
        values: &[Inline],
        mut last: u32,
    ) -> Result<u32, BookNavigationSyntaxError> {
        for inline in values {
            let id = inline.node_id();
            let end = match inline {
                Inline::Emphasis { children, .. }
                | Inline::Strong { children, .. }
                | Inline::Link { children, .. } => self.inlines(children, id)?,
                _ => id,
            };
            let text = match inline {
                Inline::Text { text_span, .. } => Some(*text_span),
                _ => None,
            };
            last = self.add(id, end, false, text)?;
        }
        Ok(last)
    }
    fn blocks(
        &mut self,
        values: &[Block],
        mut last: u32,
    ) -> Result<u32, BookNavigationSyntaxError> {
        for block in values {
            let id = block.node_id();
            let end = match block {
                Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                    self.inlines(children, id)?
                }
                Block::SemanticContainer { blocks, .. } => self.blocks(blocks, id)?,
                Block::Figure { caption, .. } | Block::VectorFigure { caption, .. } => {
                    self.blocks(caption, id)?
                }
                Block::List { items, .. } => {
                    let mut end = id;
                    for item in items {
                        end = self.blocks(&item.blocks, item.node_id)?;
                        self.add(item.node_id, end, true, None)?;
                    }
                    end
                }
                Block::DescriptionList { items, .. } => {
                    let mut end = id;
                    for item in items {
                        let term_end = self.inlines(&item.term.children, item.term.node_id)?;
                        self.add(item.term.node_id, term_end, true, None)?;
                        end = self.blocks(&item.blocks, term_end)?;
                        self.add(item.node_id, end, true, None)?;
                    }
                    end
                }
                Block::Table {
                    caption,
                    head,
                    body,
                    ..
                } => {
                    let mut end = if let Some(caption) = caption {
                        self.blocks(caption, id)?
                    } else {
                        id
                    };
                    for row in head.iter().chain(body) {
                        end = row.node_id;
                        for cell in &row.cells {
                            end = self.blocks(&cell.blocks, cell.node_id)?;
                            self.add(cell.node_id, end, false, None)?;
                        }
                        self.add(row.node_id, end, false, None)?;
                    }
                    end
                }
                Block::MathVectorBlock {
                    equation_number: Some(number),
                    ..
                } => self.add(
                    number.node_id,
                    number.node_id,
                    false,
                    Some(number.text_span),
                )?,
                _ => id,
            };
            last = self.add(id, end, !matches!(block, Block::PageBreak { .. }), None)?;
        }
        Ok(last)
    }
}

pub(super) fn prepare<'a>(
    body: &'a StyledBookV2Body,
    anchors: &mut BTreeMap<String, (u32, String)>,
    retained: &mut u64,
) -> Result<Vec<PreparedBookV2NumberBinding<'a>>, BookNavigationSyntaxError> {
    let wire = body.body().wire();
    let Some(bindings) = &wire.document().number_bindings else {
        return Ok(Vec::new());
    };
    let limits = body.body().limits();
    let path = "/document/number_bindings";
    let invalid = |i: usize| {
        BookNavigationSyntaxError::producer(
            BookNavigationSyntaxErrorKind::InvalidNumberBinding,
            format!("{path}/{i}"),
        )
    };
    let mut wanted = Vec::new();
    wanted
        .try_reserve_exact(
            bindings
                .len()
                .checked_mul(3)
                .ok_or_else(|| node_limit(path))?,
        )
        .map_err(|_| allocation(path))?;
    for binding in bindings {
        wanted.extend([binding.owner_node_id, binding.label_node_id]);
        if let Some((id, _)) = anchors.get(&binding.anchor_id) {
            wanted.push(*id);
        }
    }
    wanted.sort_unstable();
    wanted.dedup();
    let mut index = Index {
        wanted: &wanted,
        found: Vec::new(),
    };
    let document: &WireBookV2Document = wire.document();
    index.blocks(&document.blocks, document.node_id)?;
    for note in &document.footnotes {
        index.blocks(&note.blocks, note.node_id)?;
    }
    index.found.sort_unstable_by_key(|n| n.id);
    let find = |id| {
        index
            .found
            .binary_search_by_key(&id, |n| n.id)
            .ok()
            .map(|i| index.found[i])
    };
    let mut output = Vec::new();
    output
        .try_reserve_exact(bindings.len())
        .map_err(|_| allocation(path))?;
    let mut seen = BTreeSet::new();
    for (i, binding) in bindings.iter().enumerate() {
        if !seen.insert(binding.anchor_id.as_str()) {
            return Err(invalid(i));
        }
        let anchor = AnchorId::new(binding.anchor_id.clone()).map_err(|_| invalid(i))?;
        let owner = find(binding.owner_node_id)
            .filter(|n| n.can_own)
            .ok_or_else(|| invalid(i))?;
        let inside = |id| owner.id <= id && id <= owner.last;
        let label = find(binding.label_node_id)
            .filter(|n| inside(n.id))
            .ok_or_else(|| invalid(i))?;
        let source = label.text.ok_or_else(|| invalid(i))?;
        let selected = binding.text_span;
        if selected.text_id != source.text_id
            || selected.start_byte < source.start_byte
            || selected.end_byte > source.end_byte
            || selected.start_byte >= selected.end_byte
        {
            return Err(invalid(i));
        }
        let buffer = wire
            .text_buffers()
            .get(selected.text_id as usize)
            .filter(|b| b.text_id == selected.text_id)
            .ok_or_else(|| invalid(i))?;
        let text = buffer
            .utf8
            .get(selected.start_byte as usize..selected.end_byte as usize)
            .ok_or_else(|| invalid(i))?;
        if text.chars().any(char::is_control) || text.trim().is_empty() {
            return Err(invalid(i));
        }
        if let Some((id, _)) = anchors.get(binding.anchor_id.as_str()) {
            if !inside(*id) {
                return Err(invalid(i));
            }
        } else {
            anchors.insert(
                binding.anchor_id.clone(),
                (owner.id, format!("{path}/{i}/anchor_id")),
            );
        }
        charge(
            retained,
            binding.anchor_id.len() as u64,
            &format!("{path}/{i}/anchor_id"),
            limits,
        )?;
        output.push(PreparedBookV2NumberBinding {
            anchor,
            owner: NodeId::new(owner.id),
            label_owner: NodeId::new(label.id),
            text_span: selected,
            label: text,
        });
    }
    output.sort_unstable_by(|a, b| a.anchor.as_str().cmp(b.anchor.as_str()));
    Ok(output)
}
