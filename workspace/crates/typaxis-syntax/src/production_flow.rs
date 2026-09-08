//! Source-ordered input to production body/text/math layout. This is a sealed
//! syntax product, not a selected layout and not an authorization to paint.
use super::*;
use crate::ValidatedStagingBookNavigationV2;
use typaxis_document_package::WireStagingM4ReferenceFormat;

#[path = "production_table.rs"]
mod table;
pub use table::{ProductionTable, ProductionTableRow, ProductionTableCell, ProductionTableSection};

pub const PRODUCTION_TEXT_FLOW_ALGORITHM: &str = "typaxis.production-text-flow/7";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFlowErrorKind {
    ReceiptMismatch,
    InvalidStyle,
    InvalidTableGrid,
    MissingTextStyle,
    TextLimit,
    NodeLimit,
    AllocationFailure,
    MarkerOverflow,
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
            ProductionFlowErrorKind::InvalidTableGrid => "invalid_table_grid",
            ProductionFlowErrorKind::MissingTextStyle => "missing_text_style",
            ProductionFlowErrorKind::TextLimit => "text_limit",
            ProductionFlowErrorKind::NodeLimit => "node_limit",
            ProductionFlowErrorKind::AllocationFailure => "allocation_failure",
            ProductionFlowErrorKind::MarkerOverflow => "marker_overflow",
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
    link_target: Option<ProductionInlineLinkTarget<'a>>,
    reference: Option<ProductionInlineReference<'a>>,
}
/// The requested format is retained independently of any future generated label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionReferenceFormat {
    Text,
    Page,
    Number,
}

/// Borrowed source identity. Anchor owners are resolved from the same validated
/// navigation registry. Footnote IDs bind definition-order generated labels;
/// neither carrier proves that the target has been placed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionInlineReference<'a> {
    Anchor {
        target: &'a str,
        target_owner: NodeId,
        format: ProductionReferenceFormat,
    },
    Footnote {
        footnote_id: &'a str,
    },
}

/// Borrowed from the already validated package. URI spelling is preserved;
/// navigation must not reinterpret it as an internal anchor or visible text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionInlineLinkTarget<'a> {
    Internal { anchor_id: &'a str },
    Uri { uri: &'a str },
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
    pub const fn reference(&self) -> Option<ProductionInlineReference<'a>> {
        self.reference
    }
    pub const fn link_target(&self) -> Option<ProductionInlineLinkTarget<'a>> {
        self.link_target
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

/// A source figure with its computed ordinary style. Raster dimensions come
/// later from the admitted resource, never from a filename or decoded alt text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionFigure<'a> {
    owner: NodeId,
    source_span: SourceSpan,
    image_id: typaxis_core::ImageResourceId,
    alternative: &'a str,
    placement: &'a str,
    style: SemanticContainerInheritanceStyle,
    page_name: Option<typaxis_core::PageName>,
}
impl<'a> ProductionFigure<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn image_id(&self) -> typaxis_core::ImageResourceId {
        self.image_id
    }
    pub const fn alternative(&self) -> &'a str {
        self.alternative
    }
    pub const fn placement(&self) -> &'a str {
        self.placement
    }
    pub const fn style(&self) -> &SemanticContainerInheritanceStyle {
        &self.style
    }
    pub const fn page_name(&self) -> Option<&typaxis_core::PageName> {
        self.page_name.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionList {
    owner: NodeId,
    ordered: bool,
    start: Option<u32>,
    style: SemanticContainerInheritanceStyle,
    page_name: Option<typaxis_core::PageName>,
}
impl ProductionList {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn ordered(&self) -> bool {
        self.ordered
    }
    pub const fn start(&self) -> Option<u32> {
        self.start
    }
    pub const fn style(&self) -> &SemanticContainerInheritanceStyle {
        &self.style
    }
    pub const fn page_name(&self) -> Option<&typaxis_core::PageName> {
        self.page_name.as_ref()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionListItem<'a> {
    owner: NodeId,
    list_index: u32,
    item_index: u32,
    source_span: SourceSpan,
    language: &'a str,
    ordered_value: Option<u32>,
}
impl<'a> ProductionListItem<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn list_index(&self) -> u32 {
        self.list_index
    }
    pub const fn item_index(&self) -> u32 {
        self.item_index
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn language(&self) -> &'a str {
        self.language
    }
    pub const fn ordered_value(&self) -> Option<u32> {
        self.ordered_value
    }
    pub const fn key(&self) -> typaxis_core::GeneratedBufferKey {
        typaxis_core::GeneratedBufferKey::new(
            self.owner,
            typaxis_core::GenerationKind::ListMarker,
            0,
        )
    }
}

/// Definition-order marker source. Its font style is resolved through its owning flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteDefinition<'a> {
    owner: NodeId,
    id: &'a str,
    style_paragraph: Option<u32>,
    language: &'a str,
}
impl<'a> ProductionFootnoteDefinition<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn id(&self) -> &'a str {
        self.id
    }
    pub const fn style_paragraph_index(&self) -> Option<u32> {
        self.style_paragraph
    }
    pub const fn language(&self) -> &'a str {
        self.language
    }
}

/// Downstream consumers must bind to this owner and a paragraph/site index;
/// a copied `ProductionInlineSite` alone does not authorize shaping or paint.
pub type ProductionTextFlow<'a> =
    SourceTextFlow<'a, ValidatedStagingSemanticPackage, ValidatedStagingBookNavigationV2>;

/// Shared flow storage. Only the version-specific constructors can create it.
/// Its source owner types cannot be exchanged through a conversion.
#[doc(hidden)]
pub struct SourceTextFlow<'a, P, N> {
    package: &'a P,
    navigation: &'a N,
    events: Vec<ProductionFlowEvent>,
    paragraphs: Vec<ProductionTextParagraph<'a>>,
    figures: Vec<ProductionFigure<'a>>,
    tables: Vec<ProductionTable>,
    table_record_charge: u64,
    lists: Vec<ProductionList>,
    list_items: Vec<ProductionListItem<'a>>,
    footnote_definitions: Vec<ProductionFootnoteDefinition<'a>>,
    footnote_base_style: Option<SemanticContainerInheritanceStyle>,
    generated: typaxis_text::GeneratedTextOverlay,
    page_reference_values: Option<Vec<(NodeId, u32)>>,
    text_bytes: u64,
    fingerprint: [u8; 32],
}
impl<'a, P, N> SourceTextFlow<'a, P, N> {
    pub fn footnote_definitions(&self) -> &[ProductionFootnoteDefinition<'a>] {
        &self.footnote_definitions
    }
    pub fn footnote_marker_style(
        &self,
        index: usize,
    ) -> Option<&SemanticContainerInheritanceStyle> {
        let definition = self.footnote_definitions.get(index)?;
        match definition.style_paragraph {
            Some(index) => self.paragraphs.get(index as usize).map(|p| &p.style),
            None => self.footnote_base_style.as_ref(),
        }
    }
    pub fn lists(&self) -> &[ProductionList] {
        &self.lists
    }
    pub fn list_items(&self) -> &[ProductionListItem<'a>] {
        &self.list_items
    }
    pub fn list_marker_text(&self, item_index: usize) -> Option<&str> {
        let item = self.list_items.get(item_index)?;
        self.generated
            .buffer(item.key())
            .map(|buffer| buffer.utf8())
    }
    pub fn list_marker_provenance(
        &self,
        item_index: usize,
    ) -> Option<typaxis_text::GeneratedProvenance> {
        let item = self.list_items.get(item_index)?;
        let text = self.list_marker_text(item_index)?;
        self.generated
            .provenance(
                item.key(),
                typaxis_core::Utf8ByteOffset::new(0),
                typaxis_core::Utf8ByteOffset::new(u32::try_from(text.len()).ok()?),
            )
            .ok()
    }
    /// Canonical definition-order decimal label, in the generated namespace.
    /// This is not proof that the referenced footnote has been placed.
    pub fn footnote_marker_text(&self, owner: NodeId) -> Option<&str> {
        self.generated
            .buffer(footnote_marker_key(owner))
            .map(|b| b.utf8())
    }
    pub fn footnote_marker_provenance(
        &self,
        owner: NodeId,
    ) -> Option<typaxis_text::GeneratedProvenance> {
        let text = self.footnote_marker_text(owner)?;
        self.generated
            .provenance(
                footnote_marker_key(owner),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(u32::try_from(text.len()).ok()?),
            )
            .ok()
    }
    /// Candidate page labels, not evidence of final placement.
    pub fn page_reference_values(&self) -> Option<&[(NodeId, u32)]> {
        self.page_reference_values.as_deref()
    }
    pub fn page_reference_text(&self, owner: NodeId) -> Option<&str> {
        self.generated
            .buffer(page_reference_key(owner))
            .map(|b| b.utf8())
    }
    pub fn page_reference_provenance(
        &self,
        owner: NodeId,
    ) -> Option<typaxis_text::GeneratedProvenance> {
        let text = self.page_reference_text(owner)?;
        self.generated
            .provenance(
                page_reference_key(owner),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(u32::try_from(text.len()).ok()?),
            )
            .ok()
    }
    pub const fn generated_text_bytes(&self) -> u64 {
        self.generated.generated_bytes()
    }
    pub fn events(&self) -> &[ProductionFlowEvent] {
        &self.events
    }
    pub fn paragraphs(&self) -> &[ProductionTextParagraph<'a>] {
        &self.paragraphs
    }
    pub fn tables(&self) -> &[ProductionTable] {
        &self.tables
    }
    pub const fn table_record_charge(&self) -> u64 {
        self.table_record_charge
    }
    pub fn figures(&self) -> &[ProductionFigure<'a>] {
        &self.figures
    }
    pub const fn text_bytes(&self) -> u64 {
        self.text_bytes
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
impl<'a> ProductionTextFlow<'a> {
    /// The exact validated source owners retained by this flow. Downstream
    /// structure builders must use these, rather than a same-hash reparse.
    pub const fn package(&self) -> &'a ValidatedStagingSemanticPackage {
        self.package
    }
    pub const fn navigation(&self) -> &'a ValidatedStagingBookNavigationV2 {
        self.navigation
    }
    pub fn resource_declarations(&self) -> &typaxis_document::StagingM4ResourceCatalog {
        self.package.resources()
    }
    pub fn semantic_container_style(
        &self,
        owner: NodeId,
    ) -> Option<&SemanticContainerComputedStyle> {
        self.package.computed_style(owner)
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
        let observed = prepare_production_text_flow_inner(
            package,
            navigation,
            limits,
            self.page_reference_values.as_deref(),
        )?;
        if self.events != observed.events
            || self.paragraphs != observed.paragraphs
            || self.tables != observed.tables
            || self.table_record_charge != observed.table_record_charge
            || self.figures != observed.figures
            || self.lists != observed.lists
            || self.list_items != observed.list_items
            || self.footnote_definitions != observed.footnote_definitions
            || self.footnote_base_style != observed.footnote_base_style
            || self.generated != observed.generated
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
    prepare_production_text_flow_inner(package, navigation, limits, None)
}

/// Supply all Page-format reference labels in strictly increasing source-owner
/// order. Values are one-based candidate pages; pagination must later prove them.
pub fn prepare_production_text_flow_with_page_references<'a>(
    package: &'a ValidatedStagingSemanticPackage,
    navigation: &'a ValidatedStagingBookNavigationV2,
    limits: &M4EffectiveResourceLimits,
    values: &[(NodeId, u32)],
) -> Result<ProductionTextFlow<'a>, ProductionFlowError> {
    prepare_production_text_flow_inner(package, navigation, limits, Some(values))
}

fn prepare_production_text_flow_inner<'a>(
    package: &'a ValidatedStagingSemanticPackage,
    navigation: &'a ValidatedStagingBookNavigationV2,
    limits: &M4EffectiveResourceLimits,
    values: Option<&[(NodeId, u32)]>,
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
    let mut result = collect_source_flow(
        LegacyFlowSource {
            package,
            navigation,
        },
        package,
        navigation,
        wire.document(),
        wire.text_buffers(),
        rules,
        retained_production_text_bytes(package, navigation)?,
        limits.base(),
        values,
    )?;
    result.fingerprint = sha256(encode_flow(&result).as_bytes());
    Ok(result)
}

fn collect_source_flow<'a, S: FlowSource<'a>, P, N>(
    source: S,
    package: &'a P,
    navigation: &'a N,
    document: &'a WireSemanticDocument<S::Kind>,
    buffers: &'a [WireStagingM4TextBuffer],
    rules: StagingSemanticStyleSheets,
    retained_text_bytes: u64,
    limits: &ValidatedResourceLimits,
    values: Option<&[(NodeId, u32)]>,
) -> Result<SourceTextFlow<'a, P, N>, ProductionFlowError> {
    let root = NodeId::new(0);
    let mut footnote_ordinals = Vec::new();
    if document.footnotes.len() as u64 > limits.get().max_fragments {
        return Err(failure(ProductionFlowErrorKind::NodeLimit, root));
    }
    footnote_ordinals
        .try_reserve_exact(document.footnotes.len())
        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, root))?;
    for (index, footnote) in document.footnotes.iter().enumerate() {
        let ordinal = u32::try_from(index)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| {
                failure(
                    ProductionFlowErrorKind::MarkerOverflow,
                    NodeId::new(footnote.node_id),
                )
            })?;
        footnote_ordinals.push((footnote.footnote_id.as_str(), ordinal));
    }
    footnote_ordinals.sort_unstable_by_key(|(id, _)| *id);
    let mut collector = Collector {
        source,
        rules,
        buffers,
        events: Vec::new(),
        paragraphs: Vec::new(),
        figures: Vec::new(),
        tables: Vec::new(),
        table_record_charge: 0,
        lists: Vec::new(),
        list_items: Vec::new(),
        footnote_ordinals,
        generated_records: Vec::new(),
        generated_bytes: 0,
        retained_text_bytes,
        text_bytes: 0,
        node_charge: 0,
    };
    collector.blocks(&document.blocks, None)?;
    let mut footnote_definitions = Vec::new();
    footnote_definitions
        .try_reserve_exact(document.footnotes.len())
        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, root))?;
    let mut footnote_base_style = None;
    for footnote in &document.footnotes {
        let owner = NodeId::new(footnote.node_id);
        collector.add_footnote_marker(owner, &footnote.footnote_id)?;
        collector.begin(owner, ProductionFlowRegionKind::Footnote)?;
        let start = collector.paragraphs.len();
        collector.blocks(&footnote.blocks, None)?;
        let (style_paragraph, language_owner) = if let Some(first) = collector.paragraphs.get(start)
        {
            (
                Some(
                    u32::try_from(start)
                        .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, owner))?,
                ),
                first.owner(),
            )
        } else {
            if footnote_base_style.is_none() {
                footnote_base_style = Some(collector.ordinary(owner, "paragraph", &[], None)?);
            }
            (None, owner)
        };
        footnote_definitions.push(ProductionFootnoteDefinition {
            owner,
            id: &footnote.footnote_id,
            style_paragraph,
            language: collector.language(language_owner)?,
        });
        collector.end(owner)?;
    }
    let page_reference_values = add_page_reference_values(&mut collector, values, limits)?;
    let generated = typaxis_text::GeneratedTextOverlay::new(
        collector.generated_records,
        limits,
        collector.retained_text_bytes,
    )
    .map_err(|_| failure(ProductionFlowErrorKind::TextLimit, root))?;
    let result = SourceTextFlow {
        package,
        navigation,
        events: collector.events,
        paragraphs: collector.paragraphs,
        figures: collector.figures,
        tables: collector.tables,
        table_record_charge: collector.table_record_charge,
        lists: collector.lists,
        list_items: collector.list_items,
        footnote_definitions,
        footnote_base_style,
        page_reference_values,
        generated,
        text_bytes: collector.text_bytes,
        fingerprint: [0; 32],
    };
    Ok(result)
}

trait FlowSource<'a> {
    type Kind: Copy;
    fn limits(&self) -> &'a ValidatedResourceLimits;
    fn container_inheritance(&self, owner: NodeId)
        -> Option<&'a SemanticContainerInheritanceStyle>;
    fn language(&self, owner: NodeId) -> Option<&'a str>;
    fn anchors(&self) -> &'a [(AnchorId, NodeId)];
}
struct LegacyFlowSource<'a> {
    package: &'a ValidatedStagingSemanticPackage,
    navigation: &'a ValidatedStagingBookNavigationV2,
}
impl<'a> FlowSource<'a> for LegacyFlowSource<'a> {
    type Kind = typaxis_document_package::WireStagingSemanticContainerKind;
    fn limits(&self) -> &'a ValidatedResourceLimits {
        self.package.limits()
    }
    fn container_inheritance(
        &self,
        owner: NodeId,
    ) -> Option<&'a SemanticContainerInheritanceStyle> {
        self.package
            .computed_style(owner)
            .map(|style| style.inheritance_style())
    }
    fn language(&self, owner: NodeId) -> Option<&'a str> {
        self.navigation
            .languages()
            .record(owner)
            .map(|record| record.effective_language.as_ref())
    }
    fn anchors(&self) -> &'a [(AnchorId, NodeId)] {
        self.navigation.anchors()
    }
}

#[cfg(feature = "book-v2-staging")]
#[path = "book_v2_flow.rs"]
pub(super) mod book_v2;

struct Collector<'a, S: FlowSource<'a>> {
    source: S,
    rules: StagingSemanticStyleSheets,
    buffers: &'a [WireStagingM4TextBuffer],
    events: Vec<ProductionFlowEvent>,
    paragraphs: Vec<ProductionTextParagraph<'a>>,
    figures: Vec<ProductionFigure<'a>>,
    tables: Vec<ProductionTable>,
    table_record_charge: u64,
    lists: Vec<ProductionList>,
    list_items: Vec<ProductionListItem<'a>>,
    footnote_ordinals: Vec<(&'a str, u32)>,
    generated_records: Vec<(typaxis_core::GeneratedBufferKey, String)>,
    generated_bytes: u64,
    retained_text_bytes: u64,
    text_bytes: u64,
    node_charge: u64,
}
fn retained_production_text_bytes(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
) -> Result<u64, ProductionFlowError> {
    let root = NodeId::new(0);
    let wire = package
        .checked_wire()
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let languages = navigation.languages();
    let extra_language = languages
        .total_language_text_charge_bytes()
        .checked_sub(languages.prevalidated_vector_language_charge_bytes())
        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let mut total = package
        .retained_text_bytes()
        .checked_add(extra_language)
        .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, root))?;
    let metadata = wire.metadata();
    for value in [
        metadata.author.as_deref(),
        metadata.created.as_deref(),
        metadata.identifier.as_deref(),
        metadata.modified.as_deref(),
        metadata.subject.as_deref(),
        metadata.title.as_deref(),
    ]
    .into_iter()
    .flatten()
    .chain(metadata.keywords.iter().map(String::as_str))
    .chain(
        wire.outline()
            .entries
            .iter()
            .map(|entry| entry.label.as_str()),
    ) {
        total = total
            .checked_add(value.len() as u64)
            .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, root))?;
    }
    if total > package.limits().get().max_text_bytes {
        return Err(failure(ProductionFlowErrorKind::TextLimit, root));
    }
    Ok(total)
}

fn footnote_marker_key(owner: NodeId) -> typaxis_core::GeneratedBufferKey {
    typaxis_core::GeneratedBufferKey::new(owner, typaxis_core::GenerationKind::FootnoteMarker, 0)
}
impl<'a, S: FlowSource<'a>> Collector<'a, S> {
    fn add_footnote_marker(&mut self, owner: NodeId, id: &str) -> Result<(), ProductionFlowError> {
        let index = self
            .footnote_ordinals
            .binary_search_by_key(&id, |(id, _)| *id)
            .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
        let ordinal = self.footnote_ordinals[index].1;
        let bytes = u64::from(ordinal.ilog10()) + 1;
        self.generated_bytes = self
            .generated_bytes
            .checked_add(bytes)
            .filter(|n| {
                bytes <= u64::from(self.source.limits().get().max_text_buffer_bytes)
                    && self
                        .retained_text_bytes
                        .checked_add(*n)
                        .is_some_and(|total| total <= self.source.limits().get().max_text_bytes)
            })
            .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, owner))?;
        if self.generated_records.len() as u64 >= self.source.limits().get().max_fragments {
            return Err(failure(ProductionFlowErrorKind::NodeLimit, owner));
        }
        self.generated_records
            .try_reserve(1)
            .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
        self.generated_records
            .push((footnote_marker_key(owner), ordinal.to_string()));
        Ok(())
    }

    fn charge(&mut self, owner: NodeId) -> Result<(), ProductionFlowError> {
        self.node_charge = self
            .node_charge
            .checked_add(1)
            .filter(|count| *count <= self.source.limits().get().max_ast_nodes)
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
        blocks: &'a [WireSemanticBlock<S::Kind>],
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<(), ProductionFlowError> {
        use ProductionFlowRegionKind as Kind;
        for block in blocks {
            let owner = NodeId::new(block.node_id());
            let kind = match block {
                WireSemanticBlock::Paragraph { .. } => Kind::Paragraph,
                WireSemanticBlock::Heading { .. } => Kind::Heading,
                WireSemanticBlock::List { .. } => Kind::List,
                WireSemanticBlock::Table { .. } => Kind::Table,
                WireSemanticBlock::Figure { .. } => Kind::Figure,
                WireSemanticBlock::VectorFigure { .. } => Kind::VectorFigure,
                WireSemanticBlock::SemanticContainer { .. } => Kind::SemanticContainer,
                WireSemanticBlock::DisplayMath { .. } => Kind::DisplayMath,
                WireSemanticBlock::MathVectorBlock { .. } => Kind::MathVectorBlock,
                WireSemanticBlock::PageBreak { .. } => Kind::PageBreak,
            };
            self.begin(owner, kind)?;
            match block {
                WireSemanticBlock::Paragraph { children, .. }
                | WireSemanticBlock::Heading { children, .. } => {
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
                WireSemanticBlock::SemanticContainer { blocks, .. } => {
                    let style = self
                        .source
                        .container_inheritance(owner)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    self.blocks(blocks, Some(style))?;
                }
                WireSemanticBlock::List {
                    ordered,
                    start,
                    items,
                    ..
                } => {
                    let style = self.ordinary(owner, "list", block.classes(), parent)?;
                    let list_index = u32::try_from(self.lists.len())
                        .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
                    let page_name = self
                        .rules
                        .ordinary
                        .cascade_basic_document("list", block.classes())
                        .and_then(|s| s.page_name())
                        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))?;
                    self.lists
                        .try_reserve(1)
                        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
                    self.lists.push(ProductionList {
                        owner,
                        ordered: *ordered,
                        start: *start,
                        style: style.clone(),
                        page_name,
                    });
                    for (index, item) in items.iter().enumerate() {
                        let item_owner = NodeId::new(item.node_id);
                        self.begin(item_owner, Kind::ListItem)?;
                        if self.list_items.len() as u64 >= self.source.limits().get().max_fragments
                        {
                            return Err(failure(ProductionFlowErrorKind::NodeLimit, item_owner));
                        }
                        let value = if *ordered {
                            Some(
                                start
                                    .unwrap_or(1)
                                    .checked_add(u32::try_from(index).map_err(|_| {
                                        failure(ProductionFlowErrorKind::MarkerOverflow, item_owner)
                                    })?)
                                    .ok_or_else(|| {
                                        failure(ProductionFlowErrorKind::MarkerOverflow, item_owner)
                                    })?,
                            )
                        } else {
                            None
                        };
                        // Charge canonical marker bytes before allocating them;
                        // layout spacing is not appended to the marker text.
                        let size = if let Some(v) = value {
                            u64::from(v.checked_ilog10().unwrap_or(0)) + 2
                        } else {
                            3
                        };
                        self.generated_bytes = self
                            .generated_bytes
                            .checked_add(size)
                            .filter(|n| {
                                self.retained_text_bytes.checked_add(*n).is_some_and(|n| {
                                    n <= self.source.limits().get().max_text_bytes
                                }) && size
                                    <= u64::from(self.source.limits().get().max_text_buffer_bytes)
                            })
                            .ok_or_else(|| {
                                failure(ProductionFlowErrorKind::TextLimit, item_owner)
                            })?;
                        self.list_items.try_reserve(1).map_err(|_| {
                            failure(ProductionFlowErrorKind::AllocationFailure, item_owner)
                        })?;
                        if self.generated_records.len() as u64
                            >= self.source.limits().get().max_fragments
                        {
                            return Err(failure(ProductionFlowErrorKind::NodeLimit, item_owner));
                        }
                        self.generated_records.try_reserve(1).map_err(|_| {
                            failure(ProductionFlowErrorKind::AllocationFailure, item_owner)
                        })?;
                        let source = ProductionListItem {
                            owner: item_owner,
                            list_index,
                            item_index: u32::try_from(index).map_err(|_| {
                                failure(ProductionFlowErrorKind::NodeLimit, item_owner)
                            })?,
                            source_span: lower_span(item.span).map_err(|_| {
                                failure(ProductionFlowErrorKind::ReceiptMismatch, item_owner)
                            })?,
                            language: self.language(item_owner)?,
                            ordered_value: value,
                        };
                        self.generated_records.push((
                            source.key(),
                            value.map_or_else(|| "•".to_owned(), |v| format!("{v}.")),
                        ));
                        self.list_items.push(source);
                        self.blocks(&item.blocks, Some(&style))?;
                        self.end(item_owner)?;
                    }
                }
                WireSemanticBlock::Table { head, body, .. } => {
                    let style = self.ordinary(owner, "table", block.classes(), parent)?;
                    self.table(block, style.clone())?;
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
                WireSemanticBlock::Figure {
                    image_id,
                    placement,
                    alt,
                    caption,
                    ..
                } => {
                    let style = self.ordinary(owner, "figure", block.classes(), parent)?;
                    let page_name = self
                        .rules
                        .ordinary
                        .cascade_basic_document("figure", block.classes())
                        .and_then(|computed| computed.page_name())
                        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))?;
                    self.figures
                        .try_reserve(1)
                        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
                    self.figures.push(ProductionFigure {
                        owner,
                        source_span: lower_span(wire_block_span(block)).map_err(|_| {
                            failure(ProductionFlowErrorKind::ReceiptMismatch, owner)
                        })?,
                        image_id: typaxis_core::ImageResourceId::new(*image_id),
                        alternative: alt,
                        placement,
                        style: style.clone(),
                        page_name,
                    });
                    self.blocks(caption, Some(&style))?;
                }
                WireSemanticBlock::VectorFigure { caption, .. } => self.blocks(caption, parent)?,
                WireSemanticBlock::DisplayMath { .. }
                | WireSemanticBlock::MathVectorBlock { .. }
                | WireSemanticBlock::PageBreak { .. } => {}
            }
            self.end(owner)?;
        }
        Ok(())
    }
    fn language(&self, owner: NodeId) -> Result<&'a str, ProductionFlowError> {
        self.source.language(owner)
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
                        .filter(|total| *total <= self.source.limits().get().max_text_bytes)
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
                reference: match inline {
                    WireStagingM4Inline::Reference { target, format, .. } => {
                        let anchors = self.source.anchors();
                        let index = anchors
                            .binary_search_by(|(anchor, _)| anchor.as_str().cmp(target.as_str()))
                            .map_err(|_| {
                                failure(ProductionFlowErrorKind::ReceiptMismatch, owner)
                            })?;
                        let format = match format {
                            WireStagingM4ReferenceFormat::Text => ProductionReferenceFormat::Text,
                            WireStagingM4ReferenceFormat::Page => ProductionReferenceFormat::Page,
                            WireStagingM4ReferenceFormat::Number => {
                                ProductionReferenceFormat::Number
                            }
                        };
                        Some(ProductionInlineReference::Anchor {
                            target,
                            target_owner: anchors[index].1,
                            format,
                        })
                    }
                    WireStagingM4Inline::FootnoteReference { footnote_id, .. } => {
                        self.add_footnote_marker(owner, footnote_id)?;
                        Some(ProductionInlineReference::Footnote { footnote_id })
                    }
                    _ => None,
                },
                link_target: match inline {
                    WireStagingM4Inline::Link {
                        target: WireStagingM4LinkTarget::Internal { anchor_id },
                        ..
                    } => Some(ProductionInlineLinkTarget::Internal { anchor_id }),
                    WireStagingM4Inline::Link {
                        target: WireStagingM4LinkTarget::Uri { uri },
                        ..
                    } => Some(ProductionInlineLinkTarget::Uri { uri }),
                    _ => None,
                },
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
                    link_target: None,
                    reference: None,
                    ..site
                });
            }
        }
        Ok(())
    }
}

fn encode_flow(flow: &ProductionTextFlow<'_>) -> String {
    encode_source_flow(
        flow,
        PRODUCTION_TEXT_FLOW_ALGORITHM,
        flow.navigation.languages().fingerprint(),
        flow.navigation.limits().fingerprint(),
        flow.package.canonical_jcs_sha256(),
    )
}
fn encode_source_flow<P, N>(
    flow: &SourceTextFlow<'_, P, N>,
    algorithm: &str,
    language_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    package_sha256: [u8; 32],
) -> String {
    let mut s = String::from("{\"algorithm\":");
    push_jcs_string(&mut s, algorithm);
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
    s.push_str("],\"footnote_definitions\":[");
    for (index, definition) in flow.footnote_definitions.iter().enumerate() {
        if index != 0 {
            s.push(',');
        }
        s.push_str(&format!("[{},", definition.owner.get()));
        push_jcs_string(&mut s, definition.id);
        s.push(',');
        push_jcs_string(&mut s, definition.language);
        s.push(',');
        match definition.style_paragraph {
            Some(index) => s.push_str(&index.to_string()),
            None => s.push_str("null"),
        }
        s.push(']');
    }
    s.push_str("],\"generated_text_sha256\":");
    push_jcs_string(&mut s, &hex(flow.generated.reference_fingerprint().bytes()));
    s.push_str(",\"language_sha256\":");
    push_jcs_string(&mut s, &hex(language_sha256));
    s.push_str(",\"limits_sha256\":");
    push_jcs_string(&mut s, &hex(limits_sha256));
    s.push_str(",\"package_sha256\":");
    push_jcs_string(&mut s, &hex(package_sha256));
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
    include!("production_table_tests.rs");
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
    fn production_flow_charges_footnote_labels_with_all_retained_text() {
        let original = parse(COMBINED);
        let (original_limits, original_navigation) = navigation(&original);
        let original_flow =
            prepare_production_text_flow(&original, &original_navigation, &original_limits)
                .unwrap();
        let retained_text_bytes =
            retained_production_text_bytes(&original, &original_navigation).unwrap();
        assert!(
            retained_text_bytes
                > original
                    .checked_wire()
                    .unwrap()
                    .text_buffers()
                    .iter()
                    .map(|b| b.utf8.len() as u64)
                    .sum::<u64>()
        );
        let required = retained_text_bytes + original_flow.generated_text_bytes();
        for maximum in [required, required - 1] {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_text_bytes: maximum,
                max_text_buffer_bytes: maximum as u32,
                max_shaping_context_bytes: maximum as u32,
                ..ResourceLimits::default()
            })
            .unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(COMBINED, &DocumentPackageDecodePolicy::new(&limits))
                .unwrap();
            let package = StagingSemanticPackageParser::new()
                .parse(decoded, &limits)
                .unwrap();
            let (limits, navigation) = navigation(&package);
            let flow = prepare_production_text_flow(&package, &navigation, &limits);
            if maximum == required {
                assert_eq!(
                    flow.unwrap().generated_text_bytes(),
                    original_flow.generated_text_bytes()
                );
            } else {
                assert_eq!(
                    flow.err().unwrap(),
                    ProductionFlowError {
                        owner: NodeId::new(92),
                        kind: ProductionFlowErrorKind::TextLimit,
                    }
                );
            }
        }
    }

    #[test]
    fn production_flow_footnotes_use_definition_order_and_separate_generated_owners() {
        let original = parse(COMBINED);
        let mut wire = original.checked_wire().unwrap().clone();
        let mut document = wire.document().clone();
        let mut second = document.footnotes[0].clone();
        second.footnote_id = "second".into();
        second.node_id = 95;
        let WireStagingM4Block::Paragraph {
            node_id, children, ..
        } = &mut second.blocks[0]
        else {
            panic!()
        };
        *node_id = 96;
        let WireStagingM4Inline::Text { node_id, .. } = &mut children[0] else {
            panic!()
        };
        *node_id = 97;
        document.footnotes.push(second);
        for block in &mut document.blocks {
            if let WireStagingM4Block::Paragraph { children, .. } = block {
                for inline in children {
                    if let WireStagingM4Inline::Reference { node_id, span, .. } = inline {
                        *inline = WireStagingM4Inline::FootnoteReference {
                            node_id: *node_id,
                            span: *span,
                            footnote_id: "second".into(),
                            language: None,
                        };
                    }
                }
            }
        }
        wire.replace_typed_regions(document, wire.resources().clone());
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let package = parse(encoded.as_bytes());
        let (limits, navigation) = navigation(&package);
        let mut flow = prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        // The first encountered reference points to the second definition.
        for (owner, label) in [(7, "2"), (11, "1"), (92, "1"), (95, "2")] {
            let owner = NodeId::new(owner);
            assert_eq!(flow.footnote_marker_text(owner), Some(label));
            let provenance = flow.footnote_marker_provenance(owner).unwrap();
            assert_eq!(provenance.buffer_key(), footnote_marker_key(owner));
            assert_eq!(provenance.text_span().range().start_byte().get(), 0);
            assert_eq!(provenance.text_span().range().end_byte().get(), 1);
        }
        assert_eq!(flow.footnote_marker_text(NodeId::new(10)), None);
        assert_ne!(
            flow.footnote_marker_provenance(NodeId::new(7))
                .unwrap()
                .text_span()
                .text_id(),
            flow.footnote_marker_provenance(NodeId::new(95))
                .unwrap()
                .text_span()
                .text_id()
        );
        flow.verify(&package, &navigation, &limits).unwrap();
        assert_eq!(flow.footnote_definitions()[0].owner().get(), 92);
        let original_source = flow.footnote_definitions[0].clone();
        let paragraph = original_source.style_paragraph_index().unwrap() as usize;
        assert_eq!(flow.paragraphs()[paragraph].owner().get(), 93);
        assert_eq!(
            flow.footnote_marker_style(0),
            Some(flow.paragraphs()[paragraph].style())
        );
        flow.footnote_definitions[0].style_paragraph = Some(0);
        assert!(flow.verify(&package, &navigation, &limits).is_err());
        flow.footnote_definitions[0] = original_source.clone();
        flow.footnote_definitions[0].language = "und-x-tamper";
        assert!(flow.verify(&package, &navigation, &limits).is_err());
        flow.footnote_definitions[0] = original_source;
        let records = flow
            .generated
            .buffers()
            .iter()
            .map(|buffer| {
                (
                    buffer.key(),
                    if buffer.key() == footnote_marker_key(NodeId::new(7)) {
                        "1".to_owned()
                    } else {
                        buffer.utf8().to_owned()
                    },
                )
            })
            .collect();
        flow.generated =
            typaxis_text::GeneratedTextOverlay::new(records, limits.base(), 0).unwrap();
        assert_eq!(
            flow.verify(&package, &navigation, &limits)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
    }

    #[test]
    fn production_flow_retains_reference_identity_and_rejects_tampering() {
        for (wire_format, expected_format) in [
            (
                WireStagingM4ReferenceFormat::Page,
                ProductionReferenceFormat::Page,
            ),
            (
                WireStagingM4ReferenceFormat::Text,
                ProductionReferenceFormat::Text,
            ),
            (
                WireStagingM4ReferenceFormat::Number,
                ProductionReferenceFormat::Number,
            ),
        ] {
            let original = parse(COMBINED);
            let mut wire = original.checked_wire().unwrap().clone();
            let mut document = wire.document().clone();
            let mut changed = false;
            for block in &mut document.blocks {
                if let WireStagingM4Block::Paragraph { children, .. } = block {
                    for inline in children {
                        if let WireStagingM4Inline::Reference { format, .. } = inline {
                            *format = wire_format;
                            changed = true;
                        }
                    }
                }
            }
            assert!(changed);
            wire.replace_typed_regions(document, wire.resources().clone());
            let encoded = StagingSemanticDocumentPackageEncoder::new()
                .encode(&wire)
                .unwrap();
            let package = parse(encoded.as_bytes());
            let (limits, navigation) = navigation(&package);
            let mut flow = prepare_production_text_flow(&package, &navigation, &limits).unwrap();
            if expected_format == ProductionReferenceFormat::Page {
                let mut values = flow
                    .paragraphs()
                    .iter()
                    .flat_map(|p| p.items())
                    .filter(|site| {
                        matches!(
                            site.reference(),
                            Some(ProductionInlineReference::Anchor {
                                format: ProductionReferenceFormat::Page,
                                ..
                            })
                        )
                    })
                    .map(|site| (site.owner(), 12))
                    .collect::<Vec<_>>();
                values.sort_unstable_by_key(|v| v.0);
                assert!(!values.is_empty());
                let mut resolved = prepare_production_text_flow_with_page_references(
                    &package,
                    &navigation,
                    &limits,
                    &values,
                )
                .unwrap();
                resolved.verify(&package, &navigation, &limits).unwrap();
                assert_ne!(flow.fingerprint(), resolved.fingerprint());
                assert_eq!(resolved.page_reference_values(), Some(values.as_slice()));
                for &(owner, _) in &values {
                    assert_eq!(resolved.page_reference_text(owner), Some("12"));
                    assert!(resolved.page_reference_provenance(owner).is_some());
                    assert_eq!(flow.page_reference_text(owner), None);
                }
                let mut duplicate = values.clone();
                duplicate.insert(0, values[0]);
                let mut zero = values.clone();
                zero[0].1 = 0;
                let mut out_of_range = values.clone();
                out_of_range[0].1 = u32::MAX;
                let mut unknown = values.clone();
                unknown.push((NodeId::new(u32::MAX), 1));
                for invalid in [Vec::new(), duplicate, zero, out_of_range, unknown] {
                    assert_eq!(
                        prepare_production_text_flow_with_page_references(
                            &package,
                            &navigation,
                            &limits,
                            &invalid
                        )
                        .err()
                        .unwrap()
                        .kind,
                        ProductionFlowErrorKind::ReceiptMismatch
                    );
                }
                resolved.page_reference_values.as_mut().unwrap()[0].1 = 13;
                assert_eq!(
                    resolved
                        .verify(&package, &navigation, &limits)
                        .unwrap_err()
                        .kind,
                    ProductionFlowErrorKind::ReceiptMismatch
                );
            }
            let target_owner = navigation
                .anchors()
                .iter()
                .find(|(id, _)| id.as_str() == "top")
                .unwrap()
                .1;
            let expected = ProductionInlineReference::Anchor {
                target: "top",
                target_owner,
                format: expected_format,
            };
            let site = flow
                .paragraphs()
                .iter()
                .flat_map(|p| p.items())
                .find(|site| site.owner().get() == 7)
                .unwrap();
            assert_eq!(site.reference(), Some(expected));
            assert_eq!(site.content(), ProductionInlineContent::Reference);
            assert_eq!(site.link_target(), None);
            for site in flow.paragraphs().iter().flat_map(|p| p.items()) {
                match site.content() {
                    ProductionInlineContent::Reference => {
                        assert_eq!(site.reference(), Some(expected))
                    }
                    ProductionInlineContent::FootnoteReference => assert_eq!(
                        site.reference(),
                        Some(ProductionInlineReference::Footnote {
                            footnote_id: "note-1"
                        })
                    ),
                    _ => assert_eq!(site.reference(), None),
                }
            }
            flow.verify(&package, &navigation, &limits).unwrap();
            for replacement in [
                None,
                Some(ProductionInlineReference::Anchor {
                    target: "links",
                    target_owner,
                    format: expected_format,
                }),
                Some(ProductionInlineReference::Anchor {
                    target: "top",
                    target_owner: NodeId::new(6),
                    format: expected_format,
                }),
                Some(ProductionInlineReference::Anchor {
                    target: "top",
                    target_owner,
                    format: if expected_format == ProductionReferenceFormat::Page {
                        ProductionReferenceFormat::Text
                    } else {
                        ProductionReferenceFormat::Page
                    },
                }),
                Some(ProductionInlineReference::Footnote {
                    footnote_id: "note-1",
                }),
            ] {
                let site = flow
                    .paragraphs
                    .iter_mut()
                    .flat_map(|p| &mut p.items)
                    .find(|site| site.owner().get() == 7)
                    .unwrap();
                site.reference = replacement;
                assert_eq!(
                    flow.verify(&package, &navigation, &limits)
                        .unwrap_err()
                        .kind,
                    ProductionFlowErrorKind::ReceiptMismatch
                );
            }
            flow.paragraphs
                .iter_mut()
                .flat_map(|p| &mut p.items)
                .find(|site| site.owner().get() == 7)
                .unwrap()
                .reference = Some(expected);
            flow.verify(&package, &navigation, &limits).unwrap();
            flow.paragraphs
                .iter_mut()
                .flat_map(|p| &mut p.items)
                .find(|site| site.owner().get() == 11)
                .unwrap()
                .reference = Some(ProductionInlineReference::Footnote {
                footnote_id: "other-note",
            });
            assert_eq!(
                flow.verify(&package, &navigation, &limits)
                    .unwrap_err()
                    .kind,
                ProductionFlowErrorKind::ReceiptMismatch
            );
        }
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

#[path = "production_page_references.rs"]
mod production_page_references;
use production_page_references::{add_page_reference_values, page_reference_key};
