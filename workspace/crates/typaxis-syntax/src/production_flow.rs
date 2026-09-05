//! Source-ordered input to production body/text/math layout. This is a sealed
//! syntax product, not a selected layout and not an authorization to paint.
use super::*;
use crate::ValidatedStagingBookNavigationV2;

pub const PRODUCTION_TEXT_FLOW_ALGORITHM: &str = "typaxis.production-text-flow/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFlowErrorKind {
    ReceiptMismatch,
    InvalidStyle,
    MissingTextStyle,
    TextLimit,
    NodeLimit,
    AllocationFailure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFlowError {
    pub kind: ProductionFlowErrorKind,
    pub owner: NodeId,
}
impl std::fmt::Display for ProductionFlowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self.kind {
            ProductionFlowErrorKind::ReceiptMismatch => "receipt_mismatch",
            ProductionFlowErrorKind::InvalidStyle => "invalid_style",
            ProductionFlowErrorKind::MissingTextStyle => "missing_text_style",
            ProductionFlowErrorKind::TextLimit => "text_limit",
            ProductionFlowErrorKind::NodeLimit => "node_limit",
            ProductionFlowErrorKind::AllocationFailure => "allocation_failure",
        };
        write!(
            formatter,
            "production_text_flow {reason}: node {}",
            self.owner.get()
        )
    }
}
impl std::error::Error for ProductionFlowError {}
fn failure(kind: ProductionFlowErrorKind, owner: NodeId) -> ProductionFlowError {
    ProductionFlowError { kind, owner }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFlowRegionKind {
    Paragraph,
    Heading,
    List,
    ListItem,
    Table,
    TableHeadRow,
    TableBodyRow,
    TableCell,
    Figure,
    VectorFigure,
    SemanticContainer,
    DisplayMath,
    MathVectorBlock,
    PageBreak,
    Footnote,
}
impl ProductionFlowRegionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Heading => "heading",
            Self::List => "list",
            Self::ListItem => "list_item",
            Self::Table => "table",
            Self::TableHeadRow => "table_head_row",
            Self::TableBodyRow => "table_body_row",
            Self::TableCell => "table_cell",
            Self::Figure => "figure",
            Self::VectorFigure => "vector_figure",
            Self::SemanticContainer => "semantic_container",
            Self::DisplayMath => "display_math",
            Self::MathVectorBlock => "math_vector_block",
            Self::PageBreak => "page_break",
            Self::Footnote => "footnote",
        }
    }
}
/// Begin/End keep table, caption, list, semantic and footnote boundaries. In
/// particular, footnote definitions are not appended as ordinary body paragraphs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFlowEvent {
    Begin {
        owner: NodeId,
        kind: ProductionFlowRegionKind,
    },
    Paragraph {
        index: u32,
    },
    End {
        owner: NodeId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionInlineContent<'a> {
    Text { span: TextSpan, utf8: &'a str },
    NativeMath,
    InlineVector,
    MathVector,
    Reference,
    FootnoteReference,
    SoftBreak,
    HardBreak,
    Anchor,
    BeginEmphasis,
    BeginStrong,
    BeginLink,
    EndContainer,
}
impl ProductionInlineContent<'_> {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::NativeMath => "native_math",
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
            Self::Reference => "reference",
            Self::FootnoteReference => "footnote_reference",
            Self::SoftBreak => "soft_break",
            Self::HardBreak => "hard_break",
            Self::Anchor => "anchor",
            Self::BeginEmphasis => "begin_emphasis",
            Self::BeginStrong => "begin_strong",
            Self::BeginLink => "begin_link",
            Self::EndContainer => "end_container",
        }
    }
}
/// References and native math remain typed objects for their owning layout
/// stages. Their target/TeX/alternative is never substituted as visible text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionInlineSite<'a> {
    owner: NodeId,
    source_span: SourceSpan,
    language: &'a str,
    content: ProductionInlineContent<'a>,
}
impl<'a> ProductionInlineSite<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn language(&self) -> &'a str {
        self.language
    }
    pub const fn content(&self) -> ProductionInlineContent<'a> {
        self.content
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionTextParagraph<'a> {
    page_name: Option<typaxis_core::PageName>,
    owner: NodeId,
    style: SemanticContainerInheritanceStyle,
    items: Vec<ProductionInlineSite<'a>>,
}
impl<'a> ProductionTextParagraph<'a> {
    pub const fn page_name(&self) -> Option<&typaxis_core::PageName> {
        self.page_name.as_ref()
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style(&self) -> &SemanticContainerInheritanceStyle {
        &self.style
    }
    pub fn items(&self) -> &[ProductionInlineSite<'a>] {
        &self.items
    }
}

/// Downstream consumers must bind to this owner and a paragraph/site index;
/// a copied `ProductionInlineSite` alone does not authorize shaping or paint.
pub struct ProductionTextFlow<'a> {
    package: &'a ValidatedStagingSemanticPackage,
    navigation: &'a ValidatedStagingBookNavigationV2,
    events: Vec<ProductionFlowEvent>,
    paragraphs: Vec<ProductionTextParagraph<'a>>,
    text_bytes: u64,
    fingerprint: [u8; 32],
}
impl<'a> ProductionTextFlow<'a> {
    pub fn resource_declarations(&self) -> &typaxis_document::StagingM4ResourceCatalog {
        self.package.resources()
    }
    pub fn semantic_container_style(
        &self,
        owner: NodeId,
    ) -> Option<&SemanticContainerComputedStyle> {
        self.package.computed_style(owner)
    }
    pub fn events(&self) -> &[ProductionFlowEvent] {
        &self.events
    }
    pub fn paragraphs(&self) -> &[ProductionTextParagraph<'a>] {
        &self.paragraphs
    }
    pub const fn text_bytes(&self) -> u64 {
        self.text_bytes
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package.canonical_jcs_sha256()
    }
    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigationV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionFlowError> {
        let owner = NodeId::new(0);
        if !std::ptr::eq(self.package, package) || !std::ptr::eq(self.navigation, navigation) {
            return Err(failure(ProductionFlowErrorKind::ReceiptMismatch, owner));
        }
        let observed = prepare_production_text_flow(package, navigation, limits)?;
        if self.events != observed.events
            || self.paragraphs != observed.paragraphs
            || self.text_bytes != observed.text_bytes
            || self.fingerprint != observed.fingerprint
        {
            return Err(failure(ProductionFlowErrorKind::ReceiptMismatch, owner));
        }
        Ok(())
    }
}

/// Builds a separately versioned production flow without changing the frozen
/// semantic/staging receipts. All declared text sites are retained, including
/// paragraphs with no vectors. No font metrics or page coordinates are guessed.
pub fn prepare_production_text_flow<'a>(
    package: &'a ValidatedStagingSemanticPackage,
    navigation: &'a ValidatedStagingBookNavigationV2,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionTextFlow<'a>, ProductionFlowError> {
    let root = NodeId::new(0);
    navigation
        .verify(package, limits)
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let wire = package
        .checked_wire()
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let rules = lower_semantic_style_rules(wire.style_sheet(), package.limits())
        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, root))?;
    let mut collector = Collector {
        package,
        navigation,
        rules,
        buffers: wire.text_buffers(),
        events: Vec::new(),
        paragraphs: Vec::new(),
        text_bytes: 0,
        node_charge: 0,
    };
    collector.blocks(&wire.document().blocks, None)?;
    for footnote in &wire.document().footnotes {
        let owner = NodeId::new(footnote.node_id);
        collector.begin(owner, ProductionFlowRegionKind::Footnote)?;
        collector.blocks(&footnote.blocks, None)?;
        collector.end(owner)?;
    }
    let mut result = ProductionTextFlow {
        package,
        navigation,
        events: collector.events,
        paragraphs: collector.paragraphs,
        text_bytes: collector.text_bytes,
        fingerprint: [0; 32],
    };
    result.fingerprint = sha256(encode_flow(&result).as_bytes());
    Ok(result)
}

struct Collector<'a> {
    package: &'a ValidatedStagingSemanticPackage,
    navigation: &'a ValidatedStagingBookNavigationV2,
    rules: StagingSemanticStyleSheets,
    buffers: &'a [WireStagingM4TextBuffer],
    events: Vec<ProductionFlowEvent>,
    paragraphs: Vec<ProductionTextParagraph<'a>>,
    text_bytes: u64,
    node_charge: u64,
}
impl<'a> Collector<'a> {
    fn charge(&mut self, owner: NodeId) -> Result<(), ProductionFlowError> {
        self.node_charge = self
            .node_charge
            .checked_add(1)
            .filter(|count| *count <= self.package.limits().get().max_ast_nodes)
            .ok_or_else(|| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
        Ok(())
    }
    fn event(
        &mut self,
        event: ProductionFlowEvent,
        owner: NodeId,
    ) -> Result<(), ProductionFlowError> {
        self.events
            .try_reserve(1)
            .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
        self.events.push(event);
        Ok(())
    }
    fn begin(
        &mut self,
        owner: NodeId,
        kind: ProductionFlowRegionKind,
    ) -> Result<(), ProductionFlowError> {
        self.charge(owner)?;
        self.event(ProductionFlowEvent::Begin { owner, kind }, owner)
    }
    fn end(&mut self, owner: NodeId) -> Result<(), ProductionFlowError> {
        self.event(ProductionFlowEvent::End { owner }, owner)
    }
    fn ordinary(
        &self,
        owner: NodeId,
        kind: &str,
        classes: &[String],
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<SemanticContainerInheritanceStyle, ProductionFlowError> {
        cascade_staging_semantic_descendant_style(kind, classes, &self.rules.ordinary, parent)
            .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))
    }
    fn blocks(
        &mut self,
        blocks: &'a [WireStagingM4Block],
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<(), ProductionFlowError> {
        use ProductionFlowRegionKind as Kind;
        for block in blocks {
            let owner = NodeId::new(block.node_id());
            let kind = match block {
                WireStagingM4Block::Paragraph { .. } => Kind::Paragraph,
                WireStagingM4Block::Heading { .. } => Kind::Heading,
                WireStagingM4Block::List { .. } => Kind::List,
                WireStagingM4Block::Table { .. } => Kind::Table,
                WireStagingM4Block::Figure { .. } => Kind::Figure,
                WireStagingM4Block::VectorFigure { .. } => Kind::VectorFigure,
                WireStagingM4Block::SemanticContainer { .. } => Kind::SemanticContainer,
                WireStagingM4Block::DisplayMath { .. } => Kind::DisplayMath,
                WireStagingM4Block::MathVectorBlock { .. } => Kind::MathVectorBlock,
                WireStagingM4Block::PageBreak { .. } => Kind::PageBreak,
            };
            self.begin(owner, kind)?;
            match block {
                WireStagingM4Block::Paragraph { children, .. }
                | WireStagingM4Block::Heading { children, .. } => {
                    let style = self.ordinary(owner, kind.as_str(), block.classes(), parent)?;
                    let mut items = Vec::new();
                    let language = self.language(owner)?;
                    self.inlines(children, language, &mut items)?;
                    // Empty/anchor-only paragraphs need no selected font. All
                    // actual text and generated reference sites require explicit style.
                    let needs_font = items.iter().any(|item| {
                        matches!(item.content,
                        ProductionInlineContent::Text { utf8, .. } if !utf8.is_empty())
                            || matches!(
                                item.content,
                                ProductionInlineContent::Reference
                                    | ProductionInlineContent::FootnoteReference
                            )
                    });
                    if needs_font
                        && (style.font_families().is_none()
                            || style.font_size().is_none()
                            || style.line_height().is_none())
                    {
                        return Err(failure(ProductionFlowErrorKind::MissingTextStyle, owner));
                    }
                    let index = u32::try_from(self.paragraphs.len())
                        .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
                    self.paragraphs
                        .try_reserve(1)
                        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
                    let page_name = self
                        .rules
                        .ordinary
                        .cascade_basic_document(kind.as_str(), block.classes())
                        .and_then(|computed| computed.page_name())
                        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))?;
                    self.paragraphs.push(ProductionTextParagraph {
                        page_name,
                        owner,
                        style,
                        items,
                    });
                    self.event(ProductionFlowEvent::Paragraph { index }, owner)?;
                }
                WireStagingM4Block::SemanticContainer { blocks, .. } => {
                    let style = self
                        .package
                        .computed_style(owner)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    self.blocks(blocks, Some(style.inheritance_style()))?;
                }
                WireStagingM4Block::List { items, .. } => {
                    let style = self.ordinary(owner, "list", block.classes(), parent)?;
                    for item in items {
                        let item_owner = NodeId::new(item.node_id);
                        self.begin(item_owner, Kind::ListItem)?;
                        self.blocks(&item.blocks, Some(&style))?;
                        self.end(item_owner)?;
                    }
                }
                WireStagingM4Block::Table { head, body, .. } => {
                    let style = self.ordinary(owner, "table", block.classes(), parent)?;
                    for (rows, row_kind) in [(head, Kind::TableHeadRow), (body, Kind::TableBodyRow)]
                    {
                        for row in rows {
                            let row_owner = NodeId::new(row.node_id);
                            self.begin(row_owner, row_kind)?;
                            for cell in &row.cells {
                                let cell_owner = NodeId::new(cell.node_id);
                                self.begin(cell_owner, Kind::TableCell)?;
                                self.blocks(&cell.blocks, Some(&style))?;
                                self.end(cell_owner)?;
                            }
                            self.end(row_owner)?;
                        }
                    }
                }
                WireStagingM4Block::Figure { caption, .. } => {
                    let style = self.ordinary(owner, "figure", block.classes(), parent)?;
                    self.blocks(caption, Some(&style))?;
                }
                WireStagingM4Block::VectorFigure { caption, .. } => self.blocks(caption, parent)?,
                WireStagingM4Block::DisplayMath { .. }
                | WireStagingM4Block::MathVectorBlock { .. }
                | WireStagingM4Block::PageBreak { .. } => {}
            }
            self.end(owner)?;
        }
        Ok(())
    }
    fn language(&self, owner: NodeId) -> Result<&'a str, ProductionFlowError> {
        self.navigation
            .languages()
            .record(owner)
            .map(|record| record.effective_language.as_ref())
            .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))
    }
    fn inlines(
        &mut self,
        inlines: &'a [WireStagingM4Inline],
        inherited_language: &'a str,
        output: &mut Vec<ProductionInlineSite<'a>>,
    ) -> Result<(), ProductionFlowError> {
        for inline in inlines {
            let owner = NodeId::new(inline.node_id());
            self.charge(owner)?;
            let source_span = lower_span(inline.span())
                .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
            let language = if matches!(
                inline,
                WireStagingM4Inline::Anchor { .. }
                    | WireStagingM4Inline::SoftBreak { .. }
                    | WireStagingM4Inline::HardBreak { .. }
            ) {
                inherited_language
            } else {
                self.language(owner)?
            };
            let content = match inline {
                WireStagingM4Inline::Text { text_span, .. } => {
                    let buffer = self
                        .buffers
                        .get(text_span.text_id as usize)
                        .filter(|buffer| buffer.text_id == text_span.text_id)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    let utf8 = buffer
                        .utf8
                        .get(text_span.start_byte as usize..text_span.end_byte as usize)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    self.text_bytes = self
                        .text_bytes
                        .checked_add(utf8.len() as u64)
                        .filter(|total| *total <= self.package.limits().get().max_text_bytes)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, owner))?;
                    let span = TextSpan::new(
                        TextBufferId::new(text_span.text_id),
                        Utf8ByteOffset::new(text_span.start_byte),
                        Utf8ByteOffset::new(text_span.end_byte),
                    )
                    .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    ProductionInlineContent::Text { span, utf8 }
                }
                WireStagingM4Inline::InlineMath { .. } => ProductionInlineContent::NativeMath,
                WireStagingM4Inline::InlineVector { .. } => ProductionInlineContent::InlineVector,
                WireStagingM4Inline::MathVector { .. } => ProductionInlineContent::MathVector,
                WireStagingM4Inline::Reference { .. } => ProductionInlineContent::Reference,
                WireStagingM4Inline::FootnoteReference { .. } => {
                    ProductionInlineContent::FootnoteReference
                }
                WireStagingM4Inline::SoftBreak { .. } => ProductionInlineContent::SoftBreak,
                WireStagingM4Inline::HardBreak { .. } => ProductionInlineContent::HardBreak,
                WireStagingM4Inline::Anchor { .. } => ProductionInlineContent::Anchor,
                WireStagingM4Inline::Emphasis { .. } => ProductionInlineContent::BeginEmphasis,
                WireStagingM4Inline::Strong { .. } => ProductionInlineContent::BeginStrong,
                WireStagingM4Inline::Link { .. } => ProductionInlineContent::BeginLink,
            };
            let site = ProductionInlineSite {
                owner,
                source_span,
                language,
                content,
            };
            output
                .try_reserve(1)
                .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
            output.push(site);
            if let WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } = inline
            {
                self.inlines(children, language, output)?;
                output
                    .try_reserve(1)
                    .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
                output.push(ProductionInlineSite {
                    content: ProductionInlineContent::EndContainer,
                    ..site
                });
            }
        }
        Ok(())
    }
}

fn encode_flow(flow: &ProductionTextFlow<'_>) -> String {
    let mut s = String::from("{\"algorithm\":");
    push_jcs_string(&mut s, PRODUCTION_TEXT_FLOW_ALGORITHM);
    s.push_str(",\"events\":[");
    for (i, event) in flow.events.iter().enumerate() {
        if i != 0 {
            s.push(',');
        }
        match event {
            ProductionFlowEvent::Begin { owner, kind } => {
                s.push_str("[\"begin\",");
                push_jcs_string(&mut s, kind.as_str());
                s.push_str(&format!(",{}]", owner.get()));
            }
            ProductionFlowEvent::Paragraph { index } => {
                s.push_str(&format!("[\"paragraph\",{index}]"))
            }
            ProductionFlowEvent::End { owner } => s.push_str(&format!("[\"end\",{}]", owner.get())),
        }
    }
    s.push_str("],\"language_sha256\":");
    push_jcs_string(&mut s, &hex(flow.navigation.languages().fingerprint()));
    s.push_str(",\"limits_sha256\":");
    push_jcs_string(&mut s, &hex(flow.navigation.limits().fingerprint()));
    s.push_str(",\"package_sha256\":");
    push_jcs_string(&mut s, &hex(flow.package.canonical_jcs_sha256()));
    s.push_str(",\"paragraphs\":[");
    for (i, paragraph) in flow.paragraphs.iter().enumerate() {
        if i != 0 {
            s.push(',');
        }
        s.push_str("{\"font_families\":[");
        for (j, family) in paragraph
            .style
            .font_families()
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            if j != 0 {
                s.push(',');
            }
            push_jcs_string(&mut s, family);
        }
        s.push_str("],\"font_size\":");
        s.push_str(
            &paragraph
                .style
                .font_size()
                .map_or(0, |v| v.get().raw())
                .to_string(),
        );
        s.push_str(",\"items\":[");
        for (j, item) in paragraph.items.iter().enumerate() {
            if j != 0 {
                s.push(',');
            }
            s.push('[');
            push_jcs_string(&mut s, item.content.as_str());
            s.push_str(&format!(",{},", item.owner.get()));
            push_jcs_string(&mut s, item.language);
            if let ProductionInlineContent::Text { span, utf8 } = item.content {
                s.push(',');
                push_vector_text_span_jcs(&mut s, span);
                s.push(',');
                push_jcs_string(&mut s, utf8);
            }
            s.push(']');
        }
        s.push_str("],\"line_height\":");
        s.push_str(
            &paragraph
                .style
                .line_height()
                .map_or(0, |v| v.get().raw())
                .to_string(),
        );
        s.push_str(&format!(",\"owner\":{},\"page\":", paragraph.owner.get()));
        if let Some(page) = &paragraph.page_name {
            push_jcs_string(&mut s, page.as_str());
        } else {
            s.push_str("null");
        }
        s.push('}');
    }
    s.push_str(&format!("],\"text_bytes\":{}}}", flow.text_bytes));
    // Remaining block/source fields are bound by package_sha256. verify()
    // independently recomputes all styles and events, not just this hash.
    s
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::ResourceLimits;
    use typaxis_document_package::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder,
    };
    const COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
    fn parse(bytes: &[u8]) -> ValidatedStagingSemanticPackage {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap()
    }
    fn navigation(
        package: &ValidatedStagingSemanticPackage,
    ) -> (M4EffectiveResourceLimits, ValidatedStagingBookNavigationV2) {
        let limits = M4EffectiveResourceLimits::defaults_for(package.limits());
        let navigation = crate::validate_staging_book_navigation_v2(package, &limits).unwrap();
        (limits, navigation)
    }
    #[test]
    fn production_flow_keeps_body_only_paragraphs_and_mixed_content_in_source_order() {
        let package = parse(COMBINED);
        let (limits, navigation) = navigation(&package);
        let flow = prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        assert_eq!(
            flow.paragraphs()
                .iter()
                .map(|p| p.owner().get())
                .collect::<Vec<_>>(),
            [
                1, 5, 8, 14, 17, 21, 26, 29, 33, 36, 40, 44, 46, 49, 51, 53, 55, 57, 59, 66, 69,
                72, 75, 78, 83, 86, 93
            ]
        );
        let heading = &flow.paragraphs()[0];
        assert_eq!(heading.style().font_families().unwrap(), ["Collection"]);
        assert_eq!(
            heading.style().font_size().unwrap().get().raw(),
            14 * 65_536
        );
        assert_eq!(
            heading.style().line_height().unwrap().get().raw(),
            16 * 65_536
        );
        assert!(matches!(
            heading.items()[0].content(),
            ProductionInlineContent::Text {
                utf8: "Basic document",
                ..
            }
        ));
        assert_eq!(
            heading.items()[1].content(),
            ProductionInlineContent::SoftBreak
        );
        assert_eq!(
            heading.items()[2].content(),
            ProductionInlineContent::HardBreak
        );
        let tall = flow
            .paragraphs()
            .iter()
            .find(|p| p.owner().get() == 33)
            .unwrap();
        assert_eq!(tall.style().line_height().unwrap().get().raw(), 8_000_000);
        let normal = flow
            .paragraphs()
            .iter()
            .find(|p| p.owner().get() == 14)
            .unwrap();
        assert_eq!(normal.style().font_families().unwrap(), ["Body"]);
        assert_eq!(normal.style().font_size().unwrap().get().raw(), 12 * 65_536);
        let position = |id| {
            flow.events().iter().position(|event| matches!(event, ProductionFlowEvent::Begin {owner, ..} if owner.get() == id)).unwrap()
        };
        assert!(position(46) < position(48) && position(48) < position(49));
        assert!(position(78) < position(82) && position(89) < position(90));
        assert!(matches!(
            flow.events()[position(92)],
            ProductionFlowEvent::Begin {
                kind: ProductionFlowRegionKind::Footnote,
                ..
            }
        ));
        assert_eq!(
            flow.events().last(),
            Some(&ProductionFlowEvent::End {
                owner: NodeId::new(92)
            })
        );
        // Regions retain parentage rather than flattening table cells into body.
        let mut stack: Vec<(NodeId, ProductionFlowRegionKind)> = Vec::new();
        for event in flow.events() {
            match *event {
                ProductionFlowEvent::Begin { owner, kind } => {
                    if owner.get() == 26 {
                        assert_eq!(
                            stack
                                .iter()
                                .map(|(owner, _)| owner.get())
                                .collect::<Vec<_>>(),
                            [23, 24, 25]
                        );
                    }
                    stack.push((owner, kind));
                }
                ProductionFlowEvent::End { owner } => assert_eq!(stack.pop().unwrap().0, owner),
                ProductionFlowEvent::Paragraph { index } => assert_eq!(
                    stack.last().unwrap().0,
                    flow.paragraphs()[index as usize].owner()
                ),
            }
        }
        assert!(stack.is_empty());
        flow.verify(&package, &navigation, &limits).unwrap();
    }
    #[test]
    fn production_flow_preserves_inline_containers_and_never_shapes_tex_as_body_text() {
        let package = parse(COMBINED);
        let (limits, navigation) = navigation(&package);
        let flow = prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        let paragraph = |owner| {
            flow.paragraphs()
                .iter()
                .find(|p| p.owner().get() == owner)
                .unwrap()
        };
        assert_eq!(
            paragraph(5)
                .items()
                .iter()
                .map(|i| i.content().as_str())
                .collect::<Vec<_>>(),
            ["anchor", "reference"]
        );
        assert_eq!(
            paragraph(8)
                .items()
                .iter()
                .map(|i| (i.owner().get(), i.content().as_str()))
                .collect::<Vec<_>>(),
            [
                (9, "begin_link"),
                (10, "text"),
                (9, "end_container"),
                (11, "footnote_reference")
            ]
        );
        assert_eq!(
            paragraph(59)
                .items()
                .iter()
                .map(|i| (i.owner().get(), i.content().as_str()))
                .collect::<Vec<_>>(),
            [
                (60, "begin_emphasis"),
                (61, "text"),
                (60, "end_container"),
                (62, "begin_strong"),
                (63, "text"),
                (62, "end_container")
            ]
        );
        assert_eq!(
            paragraph(46).items()[0].content(),
            ProductionInlineContent::NativeMath
        );
        assert_eq!(
            paragraph(78)
                .items()
                .iter()
                .map(|i| i.content().as_str())
                .collect::<Vec<_>>(),
            ["inline_vector", "inline_vector", "math_vector"]
        );
        assert_ne!(flow.fingerprint(), [0; 32]);
        assert_eq!(
            flow.fingerprint(),
            prepare_production_text_flow(&package, &navigation, &limits)
                .unwrap()
                .fingerprint()
        );
    }
    #[test]
    fn production_flow_borrows_exact_multibyte_text_and_rejects_receipt_tampering() {
        // Same 14 UTF-8 bytes as the fixture heading; its inserted-text mapping
        // and byte extent remain valid. No normalization or character indexing.
        let utf8 = "日本語𠮷A";
        assert_eq!(utf8.len(), 14);
        let input = std::str::from_utf8(COMBINED)
            .unwrap()
            .replace("Basic document", utf8);
        let package = parse(input.as_bytes());
        let (limits, navigation) = navigation(&package);
        let mut flow = prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        let ProductionInlineContent::Text { span, utf8: actual } =
            flow.paragraphs()[0].items()[0].content()
        else {
            panic!()
        };
        assert_eq!(actual, utf8);
        assert_eq!(span.end_byte().get() - span.start_byte().get(), 14);
        let buffer = &package.checked_wire().unwrap().text_buffers()[0].utf8;
        assert_eq!(actual.as_ptr(), buffer.as_ptr());
        let other = parse(input.as_bytes());
        assert_eq!(
            flow.verify(&other, &navigation, &limits).unwrap_err().kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
        flow.events.swap(0, 1);
        assert_eq!(
            flow.verify(&package, &navigation, &limits)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
        flow.events.swap(0, 1);
        flow.paragraphs[0].items[0].content = ProductionInlineContent::Text {
            span,
            utf8: "wrong",
        };
        assert_eq!(
            flow.verify(&package, &navigation, &limits)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
    }
    #[test]
    fn production_flow_does_not_require_native_math_or_synthesize_a_missing_body_style() {
        let original = parse(COMBINED);
        let mut wire = original.checked_wire().unwrap().clone();
        let mut document = wire.document().clone();
        for block in &mut document.blocks {
            match block {
                WireStagingM4Block::Paragraph { children, .. } => {
                    for inline in children {
                        if let WireStagingM4Inline::InlineMath { node_id, span, .. } = inline {
                            *inline = WireStagingM4Inline::SoftBreak {
                                node_id: *node_id,
                                span: *span,
                            };
                        }
                    }
                }
                WireStagingM4Block::DisplayMath { node_id, span, .. } => {
                    *block = WireStagingM4Block::PageBreak {
                        node_id: *node_id,
                        span: *span,
                        classes: vec![],
                    }
                }
                _ => {}
            }
        }
        wire.replace_typed_regions(document, wire.resources().clone());
        let package = parse(
            StagingSemanticDocumentPackageEncoder::new()
                .encode(&wire)
                .unwrap()
                .as_bytes(),
        );
        let (limits, nav) = navigation(&package);
        let flow = prepare_production_text_flow(&package, &nav, &limits).unwrap();
        assert!(flow
            .paragraphs()
            .iter()
            .flat_map(|p| p.items())
            .all(|item| item.content() != ProductionInlineContent::NativeMath));
        let mut sheet = wire.style_sheet().clone();
        sheet
            .rules
            .iter_mut()
            .find(|r| r.selector == "paragraph")
            .unwrap()
            .declarations
            .retain(|d| d.name != "font_size");
        wire.replace_style_sheet(sheet);
        let package = parse(
            StagingSemanticDocumentPackageEncoder::new()
                .encode(&wire)
                .unwrap()
                .as_bytes(),
        );
        let (limits, nav) = navigation(&package);
        let error = match prepare_production_text_flow(&package, &nav, &limits) {
            Ok(_) => panic!("missing body size was guessed"),
            Err(e) => e,
        };
        assert_eq!(
            error,
            ProductionFlowError {
                kind: ProductionFlowErrorKind::MissingTextStyle,
                owner: NodeId::new(5)
            }
        );
    }
}
