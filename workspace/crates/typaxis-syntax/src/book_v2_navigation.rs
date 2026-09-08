//! Successor source navigation, owned by the exact styled body. No legacy
//! navigation, syntax, profile or layout receipt is issued here.
use super::*;
use crate::book_v2::{PreparedBookVector, StyledBookV2Body};
use typaxis_document_package::book_v2::{book_v2_wire_ast_node_count, WireBookV2Block};
use typaxis_document_package::WireStagingM4ReferenceFormat;
use typaxis_style::book_v2::BookV2SemanticContainerStyleKind;

#[derive(Debug)]
pub struct PreparedBookV2Language<'a> {
    node: NodeId,
    kind: StagingLanguageNodeKind,
    parent: Option<NodeId>,
    span: Option<SourceSpan>,
    explicit: Option<Arc<str>>,
    effective: Arc<str>,
    vector: Option<&'a PreparedBookVector>,
}
impl<'a> PreparedBookV2Language<'a> {
    pub const fn node_id(&self) -> NodeId {
        self.node
    }
    pub const fn kind(&self) -> StagingLanguageNodeKind {
        self.kind
    }
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }
    pub const fn source_span(&self) -> Option<SourceSpan> {
        self.span
    }
    pub fn explicit_language(&self) -> Option<&str> {
        self.explicit.as_deref()
    }
    pub fn effective_language(&self) -> &str {
        &self.effective
    }
    pub const fn vector(&self) -> Option<&'a PreparedBookVector> {
        self.vector
    }
}
#[derive(Debug)]
pub struct PreparedBookV2LanguageChild {
    node: NodeId,
    parent: NodeId,
    span: SourceSpan,
    effective: Arc<str>,
}
impl PreparedBookV2LanguageChild {
    pub const fn node_id(&self) -> NodeId {
        self.node
    }
    pub const fn parent(&self) -> NodeId {
        self.parent
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.span
    }
    pub fn effective_language(&self) -> &str {
        &self.effective
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookV2ReferenceTarget {
    Anchor {
        target: AnchorId,
        format: WireStagingM4ReferenceFormat,
    },
    Footnote {
        id: typaxis_core::FootnoteId,
    },
}
/// Source navigation for the successor body cannot become a legacy receipt.
///
/// ```compile_fail
/// use typaxis_syntax::{book_v2::PreparedBookV2Navigation, ValidatedStagingBookNavigationV2};
/// fn legacy(value: PreparedBookV2Navigation<'_>) -> ValidatedStagingBookNavigationV2 {
///     value
/// }
/// ```
#[derive(Debug)]
pub struct PreparedBookV2Navigation<'a> {
    body: &'a StyledBookV2Body,
    metadata: StagingDocumentMetadata,
    languages: Vec<PreparedBookV2Language<'a>>,
    children: Vec<PreparedBookV2LanguageChild>,
    anchors: Vec<(AnchorId, NodeId)>,
    internal_links: Vec<(NodeId, AnchorId)>,
    references: Vec<(NodeId, BookV2ReferenceTarget)>,
    outline: Vec<StagingOutlineEntry>,
    retained_text_bytes: u64,
}
impl<'a> PreparedBookV2Navigation<'a> {
    pub const fn body(&self) -> &'a StyledBookV2Body {
        self.body
    }
    pub const fn metadata(&self) -> &StagingDocumentMetadata {
        &self.metadata
    }
    pub fn languages(&self) -> &[PreparedBookV2Language<'a>] {
        &self.languages
    }
    pub fn language(&self, node: NodeId) -> Option<&PreparedBookV2Language<'a>> {
        self.languages
            .binary_search_by_key(&node, |r| r.node)
            .ok()
            .map(|i| &self.languages[i])
    }
    pub fn language_children(&self) -> &[PreparedBookV2LanguageChild] {
        &self.children
    }
    pub fn anchors(&self) -> &[(AnchorId, NodeId)] {
        &self.anchors
    }
    pub fn internal_links(&self) -> &[(NodeId, AnchorId)] {
        &self.internal_links
    }
    pub fn references(&self) -> &[(NodeId, BookV2ReferenceTarget)] {
        &self.references
    }
    pub fn outline(&self) -> &[StagingOutlineEntry] {
        &self.outline
    }
    pub const fn retained_text_bytes(&self) -> u64 {
        self.retained_text_bytes
    }
    pub fn semantic_kind(&self, node: NodeId) -> Option<BookV2SemanticContainerStyleKind> {
        self.body.container_style(node).map(|s| s.semantic_kind())
    }
    pub fn verify_for(&self, body: &StyledBookV2Body) -> Result<(), BookNavigationSyntaxError> {
        if std::ptr::eq(self.body, body) {
            Ok(())
        } else {
            Err(BookNavigationSyntaxError::mismatch())
        }
    }
}

/// Resolves source navigation and charges its retained text against the same
/// body budget. The borrowed owner prevents substituting a same-hash reparse.
pub fn prepare_book_v2_navigation(
    body: &StyledBookV2Body,
) -> Result<PreparedBookV2Navigation<'_>, BookNavigationSyntaxError> {
    let prepared = body.body();
    let wire = prepared.wire();
    let limits = prepared.limits();
    let wire_nodes = book_v2_wire_ast_node_count(wire, limits.get().max_ast_nesting_depth)
        .map_err(|_| BookNavigationSyntaxError::mismatch())?;
    let nodes = prepared
        .math()
        .iter()
        .try_fold(wire_nodes, |total, math| {
            total.checked_add(math.parsed().ast_node_count())
        })
        .ok_or_else(|| node_limit("/document"))?;
    if nodes > limits.get().max_ast_nodes {
        return Err(node_limit("/outline/entries"));
    }
    let metadata = validate_metadata_fields(wire.metadata(), limits)?;
    let mut total = prepared.retained_text_bytes();
    for value in [
        &metadata.author,
        &metadata.created,
        &metadata.identifier,
        &metadata.modified,
        &metadata.subject,
        &metadata.title,
    ]
    .into_iter()
    .flatten()
    {
        charge(&mut total, value.len() as u64, "/metadata", limits)?;
    }
    for keyword in &metadata.keywords {
        charge(
            &mut total,
            keyword.len() as u64,
            "/metadata/keywords",
            limits,
        )?;
    }
    let mut sites = Vec::new();
    let mut owners = BTreeMap::new();
    let mut raw_anchors = BTreeMap::new();
    collect_document(
        wire.document(),
        wire.advanced_page_masters(),
        LanguageRegistryGeneration::V2,
        &mut sites,
        &mut owners,
        &mut raw_anchors,
    )?;
    let mut pool = BTreeSet::new();
    let mut records: Vec<PreparedBookV2Language<'_>> = Vec::new();
    records
        .try_reserve_exact(sites.len())
        .map_err(|_| allocation("/document"))?;
    let mut vector_index = 0;
    for site in sites {
        let parent = site
            .parent
            .map(|parent| {
                records
                    .binary_search_by_key(&NodeId::new(parent), |r| r.node)
                    .ok()
                    .map(|i| &records[i])
                    .ok_or_else(BookNavigationSyntaxError::mismatch)
            })
            .transpose()?;
        let explicit = site
            .raw
            .as_ref()
            .map(|raw| {
                canonicalize_language(raw, &site.pointer, limits)
                    .map(|canonical| intern_language(&mut pool, canonical))
            })
            .transpose()?;
        let effective = explicit
            .clone()
            .or_else(|| parent.map(|r| r.effective.clone()))
            .ok_or_else(BookNavigationSyntaxError::mismatch)?;
        let span = site
            .span
            .map(|span| lower_span(span).map_err(|_| source_span_error(&site.pointer)))
            .transpose()?;
        if let Some(span) = span {
            let source = wire
                .sources()
                .get(span.source_id().get() as usize)
                .ok_or_else(|| source_span_error(&site.pointer))?;
            if span.end_byte().get() > source.utf8_byte_length {
                return Err(source_span_error(&site.pointer));
            }
            if let Some(parent_span) = parent.and_then(|r| r.span) {
                if span.source_id() != parent_span.source_id()
                    || span.start_byte() < parent_span.start_byte()
                    || span.end_byte() > parent_span.end_byte()
                {
                    return Err(source_span_error(&site.pointer));
                }
            }
        }
        let language_bytes = effective.len() as u64
            + site
                .raw
                .as_deref()
                .filter(|raw| *raw != effective.as_ref())
                .map_or(0, |raw| raw.len() as u64);
        let vector = if StagingComputedLanguageOwnerKindV2::from(site.kind).is_precomposed_vector()
        {
            let vector = prepared
                .vectors()
                .get(vector_index)
                .ok_or_else(BookNavigationSyntaxError::mismatch)?;
            if vector.node_id().get() != site.node_id
                || Some(vector.owner_source_span()) != span
                || language_owner_kind_for_vector(vector.kind())
                    != StagingComputedLanguageOwnerKindV2::from(site.kind)
            {
                return Err(BookNavigationSyntaxError::mismatch());
            }
            let prepaid = match (site.raw.as_deref(), vector.language()) {
                (None, None) => 0,
                (Some(raw), Some(language))
                    if raw == language.raw()
                        && effective.as_ref() == language.canonical()
                        && language.charged_bytes() == language_bytes =>
                {
                    language.charged_bytes()
                }
                _ => return Err(BookNavigationSyntaxError::mismatch()),
            };
            charge(
                &mut total,
                language_bytes
                    .checked_sub(prepaid)
                    .ok_or_else(BookNavigationSyntaxError::mismatch)?,
                &site.pointer,
                limits,
            )?;
            vector_index += 1;
            Some(vector)
        } else {
            charge(&mut total, language_bytes, &site.pointer, limits)?;
            None
        };
        records.push(PreparedBookV2Language {
            node: NodeId::new(site.node_id),
            kind: site.kind,
            parent: site.parent.map(NodeId::new),
            span,
            explicit,
            effective,
            vector,
        });
    }
    if vector_index != prepared.vectors().len() {
        return Err(BookNavigationSyntaxError::mismatch());
    }
    let mut children = Vec::new();
    for vector in prepared.vectors() {
        if let Some(number) = vector.equation_number() {
            let parent = records
                .binary_search_by_key(&vector.node_id(), |r| r.node)
                .ok()
                .map(|i| &records[i])
                .ok_or_else(BookNavigationSyntaxError::mismatch)?;
            if parent.kind != StagingLanguageNodeKind::MathVectorBlock
                || records
                    .binary_search_by_key(&number.node_id(), |r| r.node)
                    .is_ok()
            {
                return Err(BookNavigationSyntaxError::mismatch());
            }
            children
                .try_reserve(1)
                .map_err(|_| allocation("/document"))?;
            children.push(PreparedBookV2LanguageChild {
                node: number.node_id(),
                parent: vector.node_id(),
                span: number.span(),
                effective: parent.effective.clone(),
            });
        }
    }
    let mut anchors = Vec::new();
    anchors
        .try_reserve_exact(raw_anchors.len())
        .map_err(|_| allocation("/document"))?;
    for (name, (owner, _)) in &raw_anchors {
        anchors.push((
            AnchorId::new(name.clone()).map_err(|_| BookNavigationSyntaxError::mismatch())?,
            NodeId::new(*owner),
        ));
    }
    let internal_links =
        collect_internal_links(wire.document(), &anchors, LanguageRegistryGeneration::V2)?;
    let outline = validate_outline_entries(
        &wire.outline().entries,
        &owners,
        &raw_anchors,
        &|node| {
            records
                .binary_search_by_key(&node, |r| r.node)
                .ok()
                .map(|i| records[i].effective.clone())
        },
        limits,
    )?;
    for (index, entry) in outline.iter().enumerate() {
        charge(
            &mut total,
            entry.label.len() as u64,
            &format!("/outline/entries/{index}/label"),
            limits,
        )?;
    }
    let references = collect_references(wire.document(), &anchors)?;
    Ok(PreparedBookV2Navigation {
        body,
        metadata,
        languages: records,
        children,
        anchors,
        internal_links,
        references,
        outline,
        retained_text_bytes: total,
    })
}
fn charge(
    total: &mut u64,
    bytes: u64,
    pointer: &str,
    limits: &ValidatedResourceLimits,
) -> Result<(), BookNavigationSyntaxError> {
    *total = total
        .checked_add(bytes)
        .filter(|n| *n <= limits.get().max_text_bytes)
        .ok_or_else(|| {
            BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::TextAggregateLimit,
                "T2101",
                pointer,
            )
        })?;
    Ok(())
}
fn allocation(pointer: &str) -> BookNavigationSyntaxError {
    BookNavigationSyntaxError::limit(
        BookNavigationSyntaxErrorKind::AllocationFailure,
        "P1120",
        pointer,
    )
}
fn node_limit(pointer: &str) -> BookNavigationSyntaxError {
    BookNavigationSyntaxError::limit(
        BookNavigationSyntaxErrorKind::AstNodeLimit,
        "P1120",
        pointer,
    )
}

fn collect_references(
    document: &typaxis_document_package::book_v2::WireBookV2Document,
    anchors: &[(AnchorId, NodeId)],
) -> Result<Vec<(NodeId, BookV2ReferenceTarget)>, BookNavigationSyntaxError> {
    fn inlines(
        values: &[WireStagingM4Inline],
        path: &str,
        output: &mut Vec<(NodeId, BookV2ReferenceTarget, String)>,
    ) -> Result<(), BookNavigationSyntaxError> {
        for (index, value) in values.iter().enumerate() {
            let path = format!("{path}/{index}");
            let invalid = || {
                BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::InvalidOutline,
                    &path,
                )
            };
            let target = match value {
                WireStagingM4Inline::Reference { target, format, .. } => {
                    Some(BookV2ReferenceTarget::Anchor {
                        target: AnchorId::new(target.clone()).map_err(|_| invalid())?,
                        format: *format,
                    })
                }
                WireStagingM4Inline::FootnoteReference { footnote_id, .. } => {
                    Some(BookV2ReferenceTarget::Footnote {
                        id: typaxis_core::FootnoteId::new(footnote_id.clone())
                            .map_err(|_| invalid())?,
                    })
                }
                _ => None,
            };
            if let Some(target) = target {
                output.try_reserve(1).map_err(|_| allocation(&path))?;
                output.push((NodeId::new(value.node_id()), target, path.clone()));
            }
            match value {
                WireStagingM4Inline::Emphasis { children, .. }
                | WireStagingM4Inline::Strong { children, .. }
                | WireStagingM4Inline::Link { children, .. } => {
                    inlines(children, &format!("{path}/children"), output)?
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn blocks(
        values: &[WireBookV2Block],
        path: &str,
        output: &mut Vec<(NodeId, BookV2ReferenceTarget, String)>,
    ) -> Result<(), BookNavigationSyntaxError> {
        for (index, value) in values.iter().enumerate() {
            let path = format!("{path}/{index}");
            match value {
                WireBookV2Block::Paragraph { children, .. }
                | WireBookV2Block::Heading { children, .. } => {
                    inlines(children, &format!("{path}/children"), output)?
                }
                WireBookV2Block::SemanticContainer {
                    blocks: children, ..
                } => blocks(children, &format!("{path}/blocks"), output)?,
                WireBookV2Block::Figure { caption, .. }
                | WireBookV2Block::VectorFigure { caption, .. } => {
                    blocks(caption, &format!("{path}/caption"), output)?
                }
                WireBookV2Block::List { items, .. } => {
                    for (i, item) in items.iter().enumerate() {
                        blocks(&item.blocks, &format!("{path}/items/{i}/blocks"), output)?;
                    }
                }
                WireBookV2Block::Table { head, body, .. } => {
                    for (name, rows) in [("head", head), ("body", body)] {
                        for (i, row) in rows.iter().enumerate() {
                            for (j, cell) in row.cells.iter().enumerate() {
                                blocks(
                                    &cell.blocks,
                                    &format!("{path}/{name}/{i}/cells/{j}/blocks"),
                                    output,
                                )?;
                            }
                        }
                    }
                }
                WireBookV2Block::PageBreak { .. }
                | WireBookV2Block::DisplayMath { .. }
                | WireBookV2Block::MathVectorBlock { .. } => {}
            }
        }
        Ok(())
    }
    let mut definitions = BTreeSet::new();
    for (index, note) in document.footnotes.iter().enumerate() {
        typaxis_core::FootnoteId::new(note.footnote_id.clone()).map_err(|_| {
            BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("/document/footnotes/{index}/footnote_id"),
            )
        })?;
        if !definitions.insert(note.footnote_id.as_str()) {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("/document/footnotes/{index}/footnote_id"),
            ));
        }
    }
    let mut raw = Vec::new();
    blocks(&document.blocks, "/document/blocks", &mut raw)?;
    for (index, note) in document.footnotes.iter().enumerate() {
        blocks(
            &note.blocks,
            &format!("/document/footnotes/{index}/blocks"),
            &mut raw,
        )?;
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(raw.len())
        .map_err(|_| allocation("/document"))?;
    for (node, target, path) in raw {
        let exists = match &target {
            BookV2ReferenceTarget::Anchor { target, .. } => anchors
                .binary_search_by(|(anchor, _)| anchor.cmp(target))
                .is_ok(),
            BookV2ReferenceTarget::Footnote { id } => definitions.contains(id.as_str()),
        };
        if !exists {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                path,
            ));
        }
        output.push((node, target));
    }
    Ok(output)
}

#[cfg(test)]
#[path = "book_v2_navigation_tests.rs"]
mod tests;

fn source_span_error(language_pointer: &str) -> BookNavigationSyntaxError {
    let owner = language_pointer
        .strip_suffix("/language")
        .unwrap_or(language_pointer);
    BookNavigationSyntaxError::producer(
        BookNavigationSyntaxErrorKind::InvalidSourceSpan,
        format!("{owner}/span"),
    )
}
