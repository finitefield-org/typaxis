#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{
    document_fingerprint_from_jcs, push_jcs_string, style_fingerprint_from_jcs, AnchorId,
    DocumentFingerprint, FontFaceId, FootnoteId, GeneratedBufferKey, GenerationKind,
    ImageResourceId, Length, MasterId, NodeId, PageName, PortablePath, PositiveLength, Rect,
    ReferenceFingerprint, SafeUriError, SourceId, SourceSpan, StyleFingerprint, StyleId,
    TextBufferId, TextSpan, Utf8ByteOffset, Utf8ByteRange, ValidatedResourceLimits, CONTRACT,
    COORDINATE_UNIT, DEFAULT_ALLOWED_URI_SCHEMES,
};
use typaxis_diagnostics::{
    AdvisoryDiagnostic, Diagnostic, DiagnosticCode, DiagnosticFlow, ParseFailure, PhaseDiagnostics,
    Severity,
};
use typaxis_document::{
    Block, ColumnSizing, Document, DocumentNodeKind, FontFaceDeclaration, FootnoteDefinition,
    GeneratedSiteTarget, Inline, LinkTarget, ListItem, ReferenceFormat, ResourceCatalog, TableCell,
    TableColumn, TableRow, ValidatedDocumentNodeIndex,
};
use typaxis_style::{
    is_style_identifier, ComputedStyle, Declaration, PageMaster, PageMasterSet,
    PageMasterValidationError, PageParity, StyleRule, StyleSheet, StyleValidationError, StyleValue,
};
use typaxis_text::{
    GeneratedBufferDraft, GeneratedProvenance, GeneratedTextStore, SourceCatalog, SourceRecord,
    TextBuffer, TextMapKind, TextMapSegment, TextStore,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    pub source_id: SourceId,
    pub uri: PortablePath,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedPackage {
    pub sources: SourceCatalog,
    pub text_store: TextStore,
    pub document: Document,
    pub style_sheet: StyleSheet,
    pub page_masters: PageMasterSet,
    pub resources: ResourceCatalog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageValidationError {
    UnknownSource,
    SourceSpanOutOfBounds,
    SourceSpanNotUtf8Boundary,
    IdentityBytesMismatch,
    UnknownTextBuffer,
    TextSpanOutOfBounds,
    TextSpanNotUtf8Boundary,
    DuplicateNodeId,
    NonCanonicalNodeId,
    DuplicateAnchorId,
    DuplicateFootnoteId,
    UnknownInternalTarget,
    UnknownFootnoteTarget,
    DuplicateFontFaceId,
    NonCanonicalFontFaceId,
    DuplicateFontFamily,
    InvalidFontFamily,
    DuplicateImageId,
    NonCanonicalImageId,
    UnknownImageTarget,
    InvalidBlockClass,
    DuplicateBlockClass,
    NonCanonicalBlockClasses,
    InvalidStyle(StyleValidationError),
    InvalidPageMasters(PageMasterValidationError),
    InvalidUri(SafeUriError),
    InvalidListStart,
    EmptyListItems,
    ListMarkerOverflow,
    EmptyTableColumns,
    EmptyTableRows,
    InvalidTableGrid,
    TableHeadBodyCross,
    SourceByteLimit,
    InputByteLimit,
    IncludeFileLimit,
    AstNestingDepthLimit,
    AstNodeLimit,
    StyleRuleLimit,
    TextBufferByteLimit,
    TextByteLimit,
    NonCanonicalFootnoteOrder,
    MissingEntrySource,
    IncludeGraphMismatch,
    UnresolvedIncludeDirective,
}

#[derive(Clone, Debug)]
pub struct PackageValidationPolicy<'a> {
    limits: &'a ValidatedResourceLimits,
    allowed_uri_schemes: &'a [String],
}
impl<'a> PackageValidationPolicy<'a> {
    pub fn new(
        limits: &'a ValidatedResourceLimits,
        allowed_uri_schemes: &'a [String],
    ) -> Result<Self, SafeUriError> {
        let unique: BTreeSet<&str> = allowed_uri_schemes.iter().map(String::as_str).collect();
        if unique.len() != allowed_uri_schemes.len()
            || allowed_uri_schemes
                .iter()
                .any(|scheme| !DEFAULT_ALLOWED_URI_SCHEMES.contains(&scheme.as_str()))
        {
            return Err(SafeUriError::InvalidAllowedScheme);
        }
        if allowed_uri_schemes
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(SafeUriError::NonCanonicalAllowedSchemes);
        }
        Ok(Self {
            limits,
            allowed_uri_schemes,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedIncludeEdge {
    parent: SourceId,
    child: SourceId,
}
impl ResolvedIncludeEdge {
    #[allow(dead_code)] // reserved for the sealed in-crate resolver
    const fn new(parent: SourceId, child: SourceId) -> Self {
        Self { parent, child }
    }
    pub const fn parent(self) -> SourceId {
        self.parent
    }
    pub const fn child(self) -> SourceId {
        self.child
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeGraphError {
    MissingEntrySource,
    NonCanonicalEdgeOrder,
    MissingOrDuplicateParent,
    ParentNotPreviouslyResolved,
    IncludeDepthLimit,
    IncludeFileLimit,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IncludeSourceIdentity {
    source_id: SourceId,
    uri: PortablePath,
    sha256: [u8; 32],
}

/// Resolver-issued proof of entry/include closure. Every non-entry SourceId
/// has exactly one parent earlier in canonical resolver order, and its checked
/// depth is bound by the same immutable limits used for package validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedIncludeGraph {
    sources: Vec<IncludeSourceIdentity>,
    edges: Vec<ResolvedIncludeEdge>,
    max_observed_depth: u32,
}
/// In-process include resolver session owned by this crate's parser. It is not
/// public: untrusted callers cannot turn an arbitrary parent vector into a
/// trusted include-closure receipt.
///
/// ```compile_fail
/// use typaxis_syntax::IncludeResolverSession;
/// ```
#[allow(dead_code)] // production parser implementation owns this session
struct IncludeResolverSession<'a> {
    sources: &'a SourceCatalog,
    limits: &'a ValidatedResourceLimits,
    edges: Vec<ResolvedIncludeEdge>,
    depths: Vec<u32>,
    next_child: usize,
    max_observed_depth: u32,
}
impl<'a> IncludeResolverSession<'a> {
    #[allow(dead_code)] // reserved for the sealed in-crate parser owner
    fn new(
        sources: &'a SourceCatalog,
        limits: &'a ValidatedResourceLimits,
    ) -> Result<Self, IncludeGraphError> {
        if sources.records().is_empty() || sources.records()[0].source_id() != SourceId::new(0) {
            return Err(IncludeGraphError::MissingEntrySource);
        }
        let include_count = sources
            .records()
            .len()
            .checked_sub(1)
            .ok_or(IncludeGraphError::MissingEntrySource)?;
        if include_count > limits.get().max_include_files as usize {
            return Err(IncludeGraphError::IncludeFileLimit);
        }
        Ok(Self {
            sources,
            limits,
            edges: Vec::with_capacity(include_count),
            depths: vec![0u32; sources.records().len()],
            next_child: 1,
            max_observed_depth: 0,
        })
    }
    #[allow(dead_code)] // production parser implementation calls this per resolved directive
    fn admit_next_include(&mut self, parent: SourceId) -> Result<SourceId, IncludeGraphError> {
        if self.next_child >= self.sources.records().len() {
            return Err(IncludeGraphError::MissingOrDuplicateParent);
        }
        let child_value =
            u32::try_from(self.next_child).map_err(|_| IncludeGraphError::ArithmeticOverflow)?;
        let child = SourceId::new(child_value);
        if parent.get() >= child.get() {
            return Err(IncludeGraphError::ParentNotPreviouslyResolved);
        }
        let parent_depth = *self
            .depths
            .get(parent.get() as usize)
            .ok_or(IncludeGraphError::ParentNotPreviouslyResolved)?;
        let depth = parent_depth
            .checked_add(1)
            .ok_or(IncludeGraphError::ArithmeticOverflow)?;
        if depth > self.limits.get().max_include_depth {
            return Err(IncludeGraphError::IncludeDepthLimit);
        }
        self.depths[self.next_child] = depth;
        self.edges.push(ResolvedIncludeEdge::new(parent, child));
        self.next_child = self
            .next_child
            .checked_add(1)
            .ok_or(IncludeGraphError::ArithmeticOverflow)?;
        self.max_observed_depth = self.max_observed_depth.max(depth);
        Ok(child)
    }
    #[allow(dead_code)] // reserved for the sealed in-crate parser owner
    fn finish(self) -> Result<ValidatedIncludeGraph, IncludeGraphError> {
        if self.next_child != self.sources.records().len() {
            return Err(IncludeGraphError::MissingOrDuplicateParent);
        }
        let sources = self
            .sources
            .records()
            .iter()
            .map(|source| IncludeSourceIdentity {
                source_id: source.source_id(),
                uri: source.uri().clone(),
                sha256: source.content_hash(),
            })
            .collect();
        Ok(ValidatedIncludeGraph {
            sources,
            edges: self.edges,
            max_observed_depth: self.max_observed_depth,
        })
    }
}
impl ValidatedIncludeGraph {
    #[allow(dead_code)] // entry-only issuance is exposed only to fixture builds
    fn entry_only(
        sources: &SourceCatalog,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, IncludeGraphError> {
        IncludeResolverSession::new(sources, limits)?.finish()
    }
    pub const fn max_observed_depth(&self) -> u32 {
        self.max_observed_depth
    }
    pub fn edges(&self) -> &[ResolvedIncludeEdge] {
        &self.edges
    }
    fn matches(&self, sources: &SourceCatalog) -> bool {
        self.sources.len() == sources.records().len()
            && self
                .sources
                .iter()
                .zip(sources.records())
                .all(|(left, right)| {
                    left.source_id == right.source_id()
                        && left.uri == *right.uri()
                        && left.sha256 == right.content_hash()
                })
    }
}

/// Canonical document/style identities derived from the exact package
/// projections used by the portable cross-artifact validator. No API accepts
/// caller-provided digest bytes for these issued values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageEpochIdentity {
    document_jcs: String,
    style_jcs: String,
    document: DocumentFingerprint,
    style: StyleFingerprint,
}

/// Package-issued pagination inputs. The private identity fields prevent a
/// valid page-master set from being paired with another package's layout
/// epoch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagePaginationContext {
    page_masters: PageMasterSet,
    document: DocumentFingerprint,
    style: StyleFingerprint,
}
impl PackagePaginationContext {
    pub const fn page_masters(&self) -> &PageMasterSet {
        &self.page_masters
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageGeneratedTextBinding<'a> {
    package: &'a ValidatedParsedPackage,
    generated_text: &'a GeneratedTextStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageShapeTextSource {
    Parsed(TextSpan),
    Generated(GeneratedProvenance),
}

/// Canonical logical text-site identity for one paragraph. The sequence is
/// derived from the validated inline tree and is used by whole-paragraph
/// itemization to prevent callers from reordering or omitting shaping sites.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageParagraphTextSite {
    Parsed(TextSpan),
    Generated(GeneratedBufferKey),
}

/// Package-issued proof that shaping text belongs to one exact parsed or
/// selected-generated buffer and has a deterministic style context. Private
/// fields prevent a caller from pairing arbitrary bytes with another owner or
/// package fingerprint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageShapeTextReceipt<'a> {
    source: PackageShapeTextSource,
    site_owner: NodeId,
    style_owner: NodeId,
    utf8: &'a str,
    document: DocumentFingerprint,
    reference: Option<ReferenceFingerprint>,
    complete_site: bool,
    standalone_logical_text: bool,
}
impl<'a> PackageShapeTextReceipt<'a> {
    pub const fn source(&self) -> PackageShapeTextSource {
        self.source
    }
    pub const fn site_owner(&self) -> NodeId {
        self.site_owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn utf8(&self) -> &'a str {
        self.utf8
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn reference_fingerprint(&self) -> Option<ReferenceFingerprint> {
        self.reference
    }
    /// Returns whether this receipt covers the complete package-declared text
    /// site rather than a caller-selected subspan of that site.
    pub const fn covers_complete_site(&self) -> bool {
        self.complete_site
    }
    /// Returns whether package structure proves that no adjacent inline text
    /// site can contribute bidi or shaping context to this receipt.
    pub const fn is_standalone_logical_text(&self) -> bool {
        self.standalone_logical_text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageShapeTextError {
    UnknownParsedBuffer,
    InvalidSpanBoundary,
    UnownedParsedSpan,
    AmbiguousParsedSpan,
    UnknownGeneratedProvenance,
    UnknownGeneratedSite,
    MissingStyleOwner,
}

impl<'a> PackageGeneratedTextBinding<'a> {
    pub const fn package(&self) -> &'a ValidatedParsedPackage {
        self.package
    }
    pub const fn generated_text(&self) -> &'a GeneratedTextStore {
        self.generated_text
    }
    pub fn bind_generated_shape_text(
        &self,
        provenance: GeneratedProvenance,
    ) -> Result<PackageShapeTextReceipt<'a>, PackageShapeTextError> {
        if !self.generated_text.validates_provenance(provenance) {
            return Err(PackageShapeTextError::UnknownGeneratedProvenance);
        }
        let key = provenance.buffer_key();
        if self.package.document_nodes.generated_site(key).is_none() {
            return Err(PackageShapeTextError::UnknownGeneratedSite);
        }
        let style_owner = shape_style_owner(self.package.document_nodes(), key.owner())
            .ok_or(PackageShapeTextError::MissingStyleOwner)?;
        let span = provenance.text_span();
        let buffer = self
            .generated_text
            .get(span.text_id())
            .ok_or(PackageShapeTextError::UnknownGeneratedProvenance)?;
        let start = span.range().start_byte().get() as usize;
        let end = span.range().end_byte().get() as usize;
        let utf8 = buffer
            .utf8()
            .get(start..end)
            .ok_or(PackageShapeTextError::InvalidSpanBoundary)?;
        Ok(PackageShapeTextReceipt {
            source: PackageShapeTextSource::Generated(provenance),
            site_owner: key.owner(),
            style_owner,
            utf8,
            document: self.package.epoch_identity.document(),
            reference: Some(self.generated_text.reference_fingerprint()),
            complete_site: start == 0 && end == buffer.utf8().len(),
            // List markers are separate layout text with spacing represented
            // by Glue. Inline-generated text is standalone only when package
            // structure proves it is the paragraph's sole logical site.
            standalone_logical_text: key.generation_kind() == GenerationKind::ListMarker
                || (key.generation_kind() != GenerationKind::Discretionary
                    && generated_inline_site_is_standalone(
                        &self.package.package.document,
                        key.owner(),
                    )),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageGeneratedTextError {
    DocumentMismatch,
    UnknownListMarkerSite,
    ListMarkerMismatch,
    ListMarkerOverflow,
    TextBufferLimit,
    TextTotalLimit,
    ArithmeticOverflow,
    GeneratedStoreRejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageComputedStyle {
    owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    computed: ComputedStyle,
}
impl PackageComputedStyle {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn computed(&self) -> &ComputedStyle {
        &self.computed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPageSelectionName {
    owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    page_name: Option<PageName>,
}
impl ResolvedPageSelectionName {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn page_name(&self) -> Option<&PageName> {
        self.page_name.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageStyleError {
    UnknownStyleOwner,
    NonEmptyDocument,
    InvalidStyle(StyleValidationError),
}
impl PackageEpochIdentity {
    fn from_package(package: &ParsedPackage) -> Self {
        let document_jcs = encode_document_fingerprint_record(package);
        let style_jcs = encode_style_fingerprint_record(package);
        Self {
            document: document_fingerprint_from_jcs(&document_jcs),
            style: style_fingerprint_from_jcs(&style_jcs),
            document_jcs,
            style_jcs,
        }
    }
    pub const fn document(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style(&self) -> StyleFingerprint {
        self.style
    }
    pub fn document_jcs(&self) -> &str {
        &self.document_jcs
    }
    pub fn style_jcs(&self) -> &str {
        &self.style_jcs
    }
}

fn encode_document_fingerprint_record(package: &ParsedPackage) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, DocumentFingerprint::ALGORITHM_ID);
    output.push_str(",\"contract\":");
    push_jcs_string(&mut output, CONTRACT);
    output.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut output, COORDINATE_UNIT);
    output.push_str(",\"document\":");
    push_document_jcs(&mut output, &package.document);
    output.push_str(",\"resources\":");
    push_resource_catalog_jcs(&mut output, &package.resources);
    output.push_str(",\"sources\":[");
    for (index, source) in package.sources.records().iter().enumerate() {
        push_separator(&mut output, index);
        output.push_str("{\"sha256\":");
        push_hash_hex(&mut output, source.content_hash());
        output.push_str(",\"source_id\":");
        output.push_str(&source.source_id().get().to_string());
        output.push_str(",\"uri\":");
        push_jcs_string(&mut output, source.uri().as_str());
        output.push_str(",\"utf8_byte_length\":");
        output.push_str(&source.utf8_byte_length().to_string());
        output.push('}');
    }
    output.push_str("],\"text_buffers\":[");
    for (index, buffer) in package.text_store.buffers().iter().enumerate() {
        push_separator(&mut output, index);
        output.push_str("{\"mappings\":[");
        for (mapping_index, mapping) in buffer.mappings().iter().enumerate() {
            push_separator(&mut output, mapping_index);
            output.push_str("{\"kind\":");
            push_jcs_string(
                &mut output,
                match mapping.kind {
                    TextMapKind::Identity => "identity",
                    TextMapKind::Replacement => "replacement",
                    TextMapKind::Inserted => "inserted",
                },
            );
            output.push_str(",\"source_span\":");
            push_optional_source_span_jcs(&mut output, mapping.source_span);
            output.push_str(",\"text_range\":{\"end_byte\":");
            output.push_str(&mapping.text_range.end_byte().get().to_string());
            output.push_str(",\"start_byte\":");
            output.push_str(&mapping.text_range.start_byte().get().to_string());
            output.push_str("}}");
        }
        output.push_str("],\"text_id\":");
        output.push_str(&buffer.text_id().get().to_string());
        output.push_str(",\"utf8\":");
        push_jcs_string(&mut output, buffer.text());
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_style_fingerprint_record(package: &ParsedPackage) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, StyleFingerprint::ALGORITHM_ID);
    output.push_str(",\"page_masters\":");
    push_page_masters_jcs(&mut output, &package.page_masters);
    output.push_str(",\"style_sheet\":");
    push_style_sheet_jcs(&mut output, &package.style_sheet);
    output.push('}');
    output
}

fn push_resource_catalog_jcs(output: &mut String, resources: &ResourceCatalog) {
    output.push_str("{\"font_faces\":[");
    for (index, font) in resources.font_faces.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"expected_sha256\":");
        push_optional_hash_hex(output, font.expected_sha256);
        output.push_str(",\"face_index\":");
        output.push_str(&font.face_index.to_string());
        output.push_str(",\"family\":");
        push_jcs_string(output, &font.family);
        output.push_str(",\"font_face_id\":");
        output.push_str(&font.font_face_id.get().to_string());
        output.push_str(",\"uri\":");
        push_jcs_string(output, font.uri.as_str());
        output.push('}');
    }
    output.push_str("],\"images\":[");
    for (index, image) in resources.images.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"expected_sha256\":");
        push_optional_hash_hex(output, image.expected_sha256);
        output.push_str(",\"image_id\":");
        output.push_str(&image.image_id.get().to_string());
        output.push_str(",\"uri\":");
        push_jcs_string(output, image.uri.as_str());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_optional_hash_hex(output: &mut String, bytes: Option<[u8; 32]>) {
    match bytes {
        Some(bytes) => push_hash_hex(output, bytes),
        None => output.push_str("null"),
    }
}

fn push_document_jcs(output: &mut String, document: &Document) {
    output.push_str("{\"blocks\":[");
    for (index, block) in document.blocks.iter().enumerate() {
        push_separator(output, index);
        push_block_jcs(output, block);
    }
    output.push_str("],\"footnotes\":[");
    for (index, footnote) in document.footnotes.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"blocks\":[");
        for (block_index, block) in footnote.blocks.iter().enumerate() {
            push_separator(output, block_index);
            push_block_jcs(output, block);
        }
        output.push_str("],\"footnote_id\":");
        push_jcs_string(output, footnote.footnote_id.as_str());
        output.push_str(",\"node_id\":");
        output.push_str(&footnote.node_id.get().to_string());
        output.push_str(",\"span\":");
        push_source_span_jcs(output, footnote.span);
        output.push('}');
    }
    output.push_str("],\"node_id\":");
    output.push_str(&document.node_id.get().to_string());
    output.push('}');
}

fn push_block_jcs(output: &mut String, block: &Block) {
    match block {
        Block::Paragraph {
            node_id,
            span,
            classes,
            children,
        } => {
            output.push_str("{\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"kind\":\"paragraph\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::Heading {
            node_id,
            span,
            classes,
            level,
            anchor_id,
            children,
        } => {
            output.push_str("{\"anchor_id\":");
            push_optional_string_jcs(output, anchor_id.as_ref().map(AnchorId::as_str));
            output.push_str(",\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"kind\":\"heading\",\"level\":");
            output.push_str(&level.get().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::List {
            node_id,
            span,
            classes,
            ordered,
            start,
            items,
        } => {
            output.push_str("{\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"items\":[");
            for (index, item) in items.iter().enumerate() {
                push_separator(output, index);
                push_list_item_jcs(output, item);
            }
            output.push_str("],\"kind\":\"list\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"ordered\":");
            output.push_str(if *ordered { "true" } else { "false" });
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"start\":");
            push_optional_u32_jcs(output, *start);
            output.push('}');
        }
        Block::Table {
            node_id,
            span,
            classes,
            columns,
            head,
            body,
        } => {
            output.push_str("{\"body\":");
            push_table_rows_jcs(output, body);
            output.push_str(",\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"columns\":[");
            for (index, column) in columns.iter().enumerate() {
                push_separator(output, index);
                push_table_column_jcs(output, column);
            }
            output.push_str("],\"head\":");
            push_table_rows_jcs(output, head);
            output.push_str(",\"kind\":\"table\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::Figure {
            node_id,
            span,
            classes,
            image_id,
            alt,
            caption,
        } => {
            output.push_str("{\"alt\":");
            push_jcs_string(output, alt);
            output.push_str(",\"caption\":[");
            for (index, block) in caption.iter().enumerate() {
                push_separator(output, index);
                push_block_jcs(output, block);
            }
            output.push_str("],\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"image_id\":");
            output.push_str(&image_id.get().to_string());
            output.push_str(",\"kind\":\"figure\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::PageBreak {
            node_id,
            span,
            classes,
        } => {
            output.push_str("{\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"kind\":\"page_break\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
    }
}

fn push_list_item_jcs(output: &mut String, item: &ListItem) {
    output.push_str("{\"blocks\":[");
    for (index, block) in item.blocks.iter().enumerate() {
        push_separator(output, index);
        push_block_jcs(output, block);
    }
    output.push_str("],\"node_id\":");
    output.push_str(&item.node_id.get().to_string());
    output.push_str(",\"span\":");
    push_source_span_jcs(output, item.span);
    output.push('}');
}

fn push_table_column_jcs(output: &mut String, column: &TableColumn) {
    match column.sizing {
        ColumnSizing::Fixed(width) => {
            output.push_str("{\"kind\":\"fixed\",\"width\":");
            output.push_str(&width.get().raw().to_string());
        }
        ColumnSizing::Fraction(weight) => {
            output.push_str("{\"kind\":\"fraction\",\"weight\":");
            output.push_str(&weight.get().to_string());
        }
    }
    output.push('}');
}

fn push_table_rows_jcs(output: &mut String, rows: &[TableRow]) {
    output.push('[');
    for (index, row) in rows.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"cells\":[");
        for (cell_index, cell) in row.cells.iter().enumerate() {
            push_separator(output, cell_index);
            output.push_str("{\"blocks\":[");
            for (block_index, block) in cell.blocks.iter().enumerate() {
                push_separator(output, block_index);
                push_block_jcs(output, block);
            }
            output.push_str("],\"colspan\":");
            output.push_str(&cell.colspan.get().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&cell.node_id.get().to_string());
            output.push_str(",\"rowspan\":");
            output.push_str(&cell.rowspan.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, cell.span);
            output.push('}');
        }
        output.push_str("],\"node_id\":");
        output.push_str(&row.node_id.get().to_string());
        output.push_str(",\"span\":");
        push_source_span_jcs(output, row.span);
        output.push('}');
    }
    output.push(']');
}

fn push_inlines_jcs(output: &mut String, inlines: &[Inline]) {
    output.push('[');
    for (index, inline) in inlines.iter().enumerate() {
        push_separator(output, index);
        push_inline_jcs(output, inline);
    }
    output.push(']');
}

fn push_inline_jcs(output: &mut String, inline: &Inline) {
    match inline {
        Inline::Text {
            node_id,
            span,
            text_span,
        } => {
            output.push_str("{\"kind\":\"text\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"text_span\":");
            push_text_span_jcs(output, *text_span);
            output.push('}');
        }
        Inline::Emphasis {
            node_id,
            span,
            children,
        }
        | Inline::Strong {
            node_id,
            span,
            children,
        } => {
            output.push_str("{\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"kind\":");
            push_jcs_string(
                output,
                if matches!(inline, Inline::Emphasis { .. }) {
                    "emphasis"
                } else {
                    "strong"
                },
            );
            output.push_str(",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Inline::Link {
            node_id,
            span,
            target,
            children,
        } => {
            output.push_str("{\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"kind\":\"link\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"target\":");
            match target {
                LinkTarget::Internal(anchor) => {
                    output.push_str("{\"anchor_id\":");
                    push_jcs_string(output, anchor.as_str());
                    output.push_str(",\"kind\":\"internal\"}");
                }
                LinkTarget::Uri(uri) => {
                    output.push_str("{\"kind\":\"uri\",\"uri\":");
                    push_jcs_string(output, uri.as_str());
                    output.push('}');
                }
            }
            output.push('}');
        }
        Inline::Anchor {
            node_id,
            span,
            anchor_id,
        } => {
            output.push_str("{\"anchor_id\":");
            push_jcs_string(output, anchor_id.as_str());
            output.push_str(",\"kind\":\"anchor\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Inline::Reference {
            node_id,
            span,
            target,
            format,
        } => {
            output.push_str("{\"format\":");
            push_jcs_string(
                output,
                match format {
                    ReferenceFormat::Text => "text",
                    ReferenceFormat::Page => "page",
                    ReferenceFormat::Number => "number",
                },
            );
            output.push_str(",\"kind\":\"reference\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"target\":");
            push_jcs_string(output, target.as_str());
            output.push('}');
        }
        Inline::FootnoteReference {
            node_id,
            span,
            footnote_id,
        } => {
            output.push_str("{\"footnote_id\":");
            push_jcs_string(output, footnote_id.as_str());
            output.push_str(",\"kind\":\"footnote_reference\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Inline::SoftBreak { node_id, span } | Inline::HardBreak { node_id, span } => {
            output.push_str("{\"kind\":");
            push_jcs_string(
                output,
                if matches!(inline, Inline::SoftBreak { .. }) {
                    "soft_break"
                } else {
                    "hard_break"
                },
            );
            output.push_str(",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
    }
}

fn push_style_sheet_jcs(output: &mut String, style_sheet: &StyleSheet) {
    output.push_str("{\"rules\":[");
    for (index, rule) in style_sheet.rules.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"declarations\":[");
        for (declaration_index, declaration) in rule.declarations.iter().enumerate() {
            push_separator(output, declaration_index);
            output.push_str("{\"important\":");
            output.push_str(if declaration.important {
                "true"
            } else {
                "false"
            });
            output.push_str(",\"name\":");
            push_jcs_string(output, &declaration.name);
            output.push_str(",\"value\":");
            push_style_value_jcs(output, &declaration.value);
            output.push('}');
        }
        output.push_str("],\"extends\":");
        push_optional_string_jcs(output, rule.extends.as_ref().map(|value| value.as_str()));
        output.push_str(",\"selector\":");
        push_jcs_string(output, &rule.selector);
        output.push_str(",\"source_order\":");
        output.push_str(&rule.source_order.to_string());
        output.push_str(",\"style_id\":");
        push_jcs_string(output, rule.style_id.as_str());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_style_value_jcs(output: &mut String, value: &StyleValue) {
    match value {
        StyleValue::Keyword(value) => push_kind_value_jcs(output, "keyword", value),
        StyleValue::Text(value) => push_kind_value_jcs(output, "string", value),
        StyleValue::Integer(value) => {
            output.push_str("{\"kind\":\"integer\",\"value\":");
            output.push_str(&value.to_string());
            output.push('}');
        }
        StyleValue::Length(value) => {
            output.push_str("{\"kind\":\"length\",\"value\":");
            output.push_str(&value.raw().to_string());
            output.push('}');
        }
        StyleValue::Boolean(value) => {
            output.push_str("{\"kind\":\"boolean\",\"value\":");
            output.push_str(if *value { "true" } else { "false" });
            output.push('}');
        }
        StyleValue::FontFamilyList(families) => {
            output.push_str("{\"families\":");
            push_strings_jcs(output, families);
            output.push_str(",\"kind\":\"font_family_list\"}");
        }
        StyleValue::Ratio {
            numerator,
            denominator,
        } => {
            output.push_str("{\"denominator\":");
            output.push_str(&denominator.get().to_string());
            output.push_str(",\"kind\":\"ratio\",\"numerator\":");
            output.push_str(&numerator.to_string());
            output.push('}');
        }
    }
}

fn push_kind_value_jcs(output: &mut String, kind: &str, value: &str) {
    output.push_str("{\"kind\":");
    push_jcs_string(output, kind);
    output.push_str(",\"value\":");
    push_jcs_string(output, value);
    output.push('}');
}

fn push_page_masters_jcs(output: &mut String, page_masters: &PageMasterSet) {
    output.push_str("{\"default_master_id\":");
    push_jcs_string(output, page_masters.default_master_id.as_str());
    output.push_str(",\"masters\":[");
    for (index, master) in page_masters.masters.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"body\":");
        push_rect_jcs(output, master.body);
        output.push_str(",\"footer\":");
        push_optional_rect_jcs(output, master.footer);
        output.push_str(",\"footnote\":");
        push_optional_rect_jcs(output, master.footnote);
        output.push_str(",\"header\":");
        push_optional_rect_jcs(output, master.header);
        output.push_str(",\"height\":");
        output.push_str(&master.height.get().raw().to_string());
        output.push_str(",\"master_id\":");
        push_jcs_string(output, master.master_id.as_str());
        output.push_str(",\"width\":");
        output.push_str(&master.width.get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"selection_rules\":[");
    for (index, rule) in page_masters.selection_rules.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"first\":");
        match rule.first {
            Some(value) => output.push_str(if value { "true" } else { "false" }),
            None => output.push_str("null"),
        }
        output.push_str(",\"master_id\":");
        push_jcs_string(output, rule.master_id.as_str());
        output.push_str(",\"named_page\":");
        push_optional_string_jcs(output, rule.named_page.as_ref().map(|value| value.as_str()));
        output.push_str(",\"parity\":");
        push_jcs_string(
            output,
            match rule.parity {
                PageParity::Any => "any",
                PageParity::Odd => "odd",
                PageParity::Even => "even",
            },
        );
        output.push_str(",\"source_order\":");
        output.push_str(&rule.source_order.to_string());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_source_span_jcs(output: &mut String, span: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&span.start_byte().get().to_string());
    output.push('}');
}

fn push_optional_source_span_jcs(output: &mut String, span: Option<SourceSpan>) {
    match span {
        Some(span) => push_source_span_jcs(output, span),
        None => output.push_str("null"),
    }
}

fn push_text_span_jcs(output: &mut String, span: TextSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&span.range().end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&span.range().start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&span.text_id().get().to_string());
    output.push('}');
}

fn push_rect_jcs(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

fn push_optional_rect_jcs(output: &mut String, rect: Option<Rect>) {
    match rect {
        Some(rect) => push_rect_jcs(output, rect),
        None => output.push_str("null"),
    }
}

fn push_strings_jcs(output: &mut String, values: &[String]) {
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        push_separator(output, index);
        push_jcs_string(output, value);
    }
    output.push(']');
}

fn push_optional_string_jcs(output: &mut String, value: Option<&str>) {
    match value {
        Some(value) => push_jcs_string(output, value),
        None => output.push_str("null"),
    }
}

fn push_optional_u32_jcs(output: &mut String, value: Option<u32>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_separator(output: &mut String, index: usize) {
    if index > 0 {
        output.push(',');
    }
}

fn push_hash_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

/// A parsed package that has crossed the syntax phase's validation boundary.
/// Arbitrary `ParsedPackage` values cannot be promoted through a feature flag:
///
/// ```compile_fail
/// use typaxis_syntax::ValidatedParsedPackage;
/// let _ = ValidatedParsedPackage::new_entry_only;
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedParsedPackage {
    package: ParsedPackage,
    document_nodes: ValidatedDocumentNodeIndex,
    include_graph: ValidatedIncludeGraph,
    epoch_identity: PackageEpochIdentity,
}
impl ValidatedParsedPackage {
    /// Validates an entry-only parser result. The syntax keyword scan prevents
    /// a caller from claiming entry-only closure while the admitted entry still
    /// contains an unresolved include directive. Multi-source issuance remains
    /// owned by the in-crate include resolver.
    #[cfg(test)]
    fn new_entry_only(
        package: ParsedPackage,
        policy: &PackageValidationPolicy<'_>,
    ) -> Result<Self, PackageValidationError> {
        if package.sources.records().len() != 1 {
            return Err(PackageValidationError::IncludeGraphMismatch);
        }
        if contains_include_directive(package.sources.records()[0].utf8()) {
            return Err(PackageValidationError::UnresolvedIncludeDirective);
        }
        let include_graph = ValidatedIncludeGraph::entry_only(&package.sources, policy.limits)
            .map_err(|_| PackageValidationError::IncludeGraphMismatch)?;
        Self::new_resolved(package, policy, &include_graph)
    }

    #[allow(dead_code)] // called by the in-crate parser owner once implemented
    fn new_resolved(
        package: ParsedPackage,
        policy: &PackageValidationPolicy<'_>,
        include_graph: &ValidatedIncludeGraph,
    ) -> Result<Self, PackageValidationError> {
        if !include_graph.matches(&package.sources)
            || include_graph.max_observed_depth() > policy.limits.get().max_include_depth
        {
            return Err(PackageValidationError::IncludeGraphMismatch);
        }
        let non_document_ast_nodes = validate_package_limits(&package, policy)?;
        validate_document_ast_limits(&package.document, policy.limits, non_document_ast_nodes)?;
        for buffer in package.text_store.buffers() {
            for mapping in buffer.mappings() {
                if let Some(source_span) = mapping.source_span {
                    let source = validate_source_span(&package, source_span)?;
                    if mapping.kind == TextMapKind::Identity {
                        let text_start = mapping.text_range.start_byte().get() as usize;
                        let text_end = mapping.text_range.end_byte().get() as usize;
                        let source_start = source_span.start_byte().get() as usize;
                        let source_end = source_span.end_byte().get() as usize;
                        if buffer.text()[text_start..text_end]
                            != source.utf8()[source_start..source_end]
                        {
                            return Err(PackageValidationError::IdentityBytesMismatch);
                        }
                    }
                }
            }
        }
        validate_style_inheritance_depth(&package.style_sheet, policy.limits)?;
        package
            .style_sheet
            .validate()
            .map_err(PackageValidationError::InvalidStyle)?;
        package
            .page_masters
            .validate()
            .map_err(PackageValidationError::InvalidPageMasters)?;
        let image_ids = validate_resource_catalog(&package.resources)?;
        validate_document(&package, &image_ids, policy, non_document_ast_nodes)?;
        let document_nodes = ValidatedDocumentNodeIndex::new(&package.document)
            .map_err(|_| PackageValidationError::NonCanonicalNodeId)?;
        let epoch_identity = PackageEpochIdentity::from_package(&package);
        Ok(Self {
            package,
            document_nodes,
            include_graph: include_graph.clone(),
            epoch_identity,
        })
    }

    pub fn package(&self) -> &ParsedPackage {
        &self.package
    }
    pub const fn document_nodes(&self) -> &ValidatedDocumentNodeIndex {
        &self.document_nodes
    }
    pub const fn include_graph(&self) -> &ValidatedIncludeGraph {
        &self.include_graph
    }
    pub const fn epoch_identity(&self) -> &PackageEpochIdentity {
        &self.epoch_identity
    }
    pub fn pagination_context(&self) -> PackagePaginationContext {
        PackagePaginationContext {
            page_masters: self.package.page_masters.clone(),
            document: self.epoch_identity.document(),
            style: self.epoch_identity.style(),
        }
    }
    /// Materializes the canonical Profile 1.0 bytes for one registered list
    /// marker. Ordered markers are ASCII decimal plus `.`; unordered markers
    /// are U+2022. Marker-adjacent spacing remains layout Glue.
    pub fn materialize_list_marker(
        &self,
        key: GeneratedBufferKey,
    ) -> Result<GeneratedBufferDraft, PackageGeneratedTextError> {
        if key.generation_kind() != GenerationKind::ListMarker || key.owner_local_ordinal() != 0 {
            return Err(PackageGeneratedTextError::UnknownListMarkerSite);
        }
        let markers = canonical_list_marker_texts(&self.package.document)?;
        let utf8 = markers
            .get(&key.owner())
            .ok_or(PackageGeneratedTextError::UnknownListMarkerSite)?
            .clone();
        GeneratedBufferDraft::new(&self.document_nodes, key, utf8)
            .map_err(|_| PackageGeneratedTextError::UnknownListMarkerSite)
    }
    /// Builds the deterministic state-0 generated-text overlay solely from
    /// validated package facts. State-dependent references begin empty;
    /// list and footnote markers are canonical package-derived text, and
    /// explicit soft/hard-break discretionary sites begin empty.
    pub fn materialize_initial_generated_text(
        &self,
        limits: &ValidatedResourceLimits,
    ) -> Result<GeneratedTextStore, PackageGeneratedTextError> {
        let footnote_numbers: BTreeMap<_, _> = self
            .package
            .document
            .footnotes
            .iter()
            .enumerate()
            .map(|(index, footnote)| {
                let number = index
                    .checked_add(1)
                    .ok_or(PackageGeneratedTextError::ArithmeticOverflow)?;
                Ok((footnote.footnote_id.clone(), number.to_string()))
            })
            .collect::<Result<_, PackageGeneratedTextError>>()?;
        let mut drafts = Vec::new();
        drafts
            .try_reserve_exact(self.document_nodes.generated_sites().len())
            .map_err(|_| PackageGeneratedTextError::GeneratedStoreRejected)?;
        for site in self.document_nodes.generated_sites() {
            let key = site.key();
            let utf8 = match key.generation_kind() {
                GenerationKind::ListMarker => {
                    drafts.push(self.materialize_list_marker(key)?);
                    continue;
                }
                GenerationKind::FootnoteMarker => match site.target() {
                    GeneratedSiteTarget::Footnote(footnote_id) => footnote_numbers
                        .get(footnote_id)
                        .cloned()
                        .ok_or(PackageGeneratedTextError::GeneratedStoreRejected)?,
                    GeneratedSiteTarget::None => {
                        let footnote_id = self
                            .package
                            .document
                            .footnotes
                            .iter()
                            .find(|footnote| footnote.node_id == key.owner())
                            .map(|footnote| &footnote.footnote_id)
                            .ok_or(PackageGeneratedTextError::GeneratedStoreRejected)?;
                        footnote_numbers
                            .get(footnote_id)
                            .cloned()
                            .ok_or(PackageGeneratedTextError::GeneratedStoreRejected)?
                    }
                    GeneratedSiteTarget::Anchor(_) => {
                        return Err(PackageGeneratedTextError::GeneratedStoreRejected)
                    }
                },
                GenerationKind::PageReference
                | GenerationKind::Counter
                | GenerationKind::Discretionary => String::new(),
            };
            drafts.push(
                GeneratedBufferDraft::new(&self.document_nodes, key, utf8)
                    .map_err(|_| PackageGeneratedTextError::GeneratedStoreRejected)?,
            );
        }
        GeneratedTextStore::new(
            drafts,
            &self.document_nodes,
            limits,
            &self.package.text_store,
        )
        .map_err(|_| PackageGeneratedTextError::GeneratedStoreRejected)
    }
    pub fn bind_generated_text<'a>(
        &'a self,
        generated_text: &'a GeneratedTextStore,
        limits: &ValidatedResourceLimits,
    ) -> Result<PackageGeneratedTextBinding<'a>, PackageGeneratedTextError> {
        if generated_text.document_nodes() != self.document_nodes() {
            return Err(PackageGeneratedTextError::DocumentMismatch);
        }
        let list_markers = canonical_list_marker_texts(&self.package.document)?;
        let limits = limits.get();
        let mut total = 0u64;
        for buffer in self.package.text_store.buffers() {
            let bytes = u64::from(buffer.byte_len());
            if bytes > u64::from(limits.max_text_buffer_bytes) {
                return Err(PackageGeneratedTextError::TextBufferLimit);
            }
            total = total
                .checked_add(bytes)
                .ok_or(PackageGeneratedTextError::ArithmeticOverflow)?;
        }
        for buffer in generated_text.buffers() {
            if buffer.key().generation_kind() == GenerationKind::ListMarker
                && list_markers.get(&buffer.key().owner()).map(String::as_str)
                    != Some(buffer.utf8())
            {
                return Err(PackageGeneratedTextError::ListMarkerMismatch);
            }
            let bytes = u64::try_from(buffer.utf8().len())
                .map_err(|_| PackageGeneratedTextError::ArithmeticOverflow)?;
            if bytes > u64::from(limits.max_text_buffer_bytes) {
                return Err(PackageGeneratedTextError::TextBufferLimit);
            }
            total = total
                .checked_add(bytes)
                .ok_or(PackageGeneratedTextError::ArithmeticOverflow)?;
        }
        if total > limits.max_text_bytes {
            return Err(PackageGeneratedTextError::TextTotalLimit);
        }
        Ok(PackageGeneratedTextBinding {
            package: self,
            generated_text,
        })
    }
    pub fn bind_parsed_shape_text(
        &self,
        span: TextSpan,
    ) -> Result<PackageShapeTextReceipt<'_>, PackageShapeTextError> {
        let buffer = self
            .package
            .text_store
            .get(span.text_id())
            .ok_or(PackageShapeTextError::UnknownParsedBuffer)?;
        let start = span.start_byte().get() as usize;
        let end = span.end_byte().get() as usize;
        let utf8 = buffer
            .text()
            .get(start..end)
            .ok_or(PackageShapeTextError::InvalidSpanBoundary)?;
        let (site_owner, style_owner, declared_span, standalone_logical_text) =
            parsed_shape_owners(&self.package.document, span)?;
        Ok(PackageShapeTextReceipt {
            source: PackageShapeTextSource::Parsed(span),
            site_owner,
            style_owner,
            utf8,
            document: self.epoch_identity.document(),
            reference: None,
            complete_site: span == declared_span,
            standalone_logical_text,
        })
    }
    pub fn cascade_style(&self, owner: NodeId) -> Result<PackageComputedStyle, PackageStyleError> {
        let (style_owner, block_type, classes) =
            find_styleable_block(&self.package.document, owner)
                .ok_or(PackageStyleError::UnknownStyleOwner)?;
        let computed = self
            .package
            .style_sheet
            .cascade(block_type, classes)
            .map_err(PackageStyleError::InvalidStyle)?;
        Ok(PackageComputedStyle {
            owner,
            style_owner,
            document: self.epoch_identity.document(),
            style: self.epoch_identity.style(),
            computed,
        })
    }

    pub fn paragraph_shape_text_sites(
        &self,
        paragraph_owner: NodeId,
    ) -> Option<Vec<PackageParagraphTextSite>> {
        paragraph_inline_children(&self.package.document, paragraph_owner).map(|children| {
            let mut sites = Vec::new();
            collect_shape_text_site_identities(children, &mut sites);
            sites
        })
    }
    pub fn resolve_page_selection(
        &self,
        owner: NodeId,
    ) -> Result<ResolvedPageSelectionName, PackageStyleError> {
        let computed = self.cascade_style(owner)?;
        let page_name = computed
            .computed
            .page_name()
            .map_err(PackageStyleError::InvalidStyle)?;
        Ok(ResolvedPageSelectionName {
            owner,
            style_owner: computed.style_owner,
            document: computed.document,
            style: computed.style,
            page_name,
        })
    }
    /// Issues the `auto` page selection only for the canonical blank-document
    /// case. Non-empty flow must resolve the `page` property for its owner.
    pub fn resolve_blank_page_selection(
        &self,
    ) -> Result<ResolvedPageSelectionName, PackageStyleError> {
        if !self.package.document.blocks.is_empty() || !self.package.document.footnotes.is_empty() {
            return Err(PackageStyleError::NonEmptyDocument);
        }
        Ok(ResolvedPageSelectionName {
            owner: self.package.document.node_id,
            style_owner: self.package.document.node_id,
            document: self.epoch_identity.document(),
            style: self.epoch_identity.style(),
            page_name: None,
        })
    }
    pub fn into_package(self) -> ParsedPackage {
        self.package
    }
}

fn canonical_list_marker_texts(
    document: &Document,
) -> Result<BTreeMap<NodeId, String>, PackageGeneratedTextError> {
    let mut markers = BTreeMap::new();
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .flat_map(|footnote| footnote.blocks.iter())
        .chain(document.blocks.iter())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::List {
                ordered,
                start,
                items,
                ..
            } => {
                for (index, item) in items.iter().enumerate() {
                    let marker = if *ordered {
                        let start = start.ok_or(PackageGeneratedTextError::ListMarkerOverflow)?;
                        let offset = u32::try_from(index)
                            .map_err(|_| PackageGeneratedTextError::ListMarkerOverflow)?;
                        let value = start
                            .checked_add(offset)
                            .ok_or(PackageGeneratedTextError::ListMarkerOverflow)?;
                        format!("{value}.")
                    } else {
                        "\u{2022}".to_owned()
                    };
                    markers.insert(item.node_id, marker);
                    pending.extend(item.blocks.iter());
                }
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    head.iter()
                        .chain(body)
                        .flat_map(|row| row.cells.iter())
                        .flat_map(|cell| cell.blocks.iter()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption),
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    Ok(markers)
}

fn find_styleable_block(
    document: &Document,
    owner: NodeId,
) -> Option<(NodeId, &'static str, &[String])> {
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        let (node_id, block_type) = match block {
            Block::Paragraph { node_id, .. } => (*node_id, "paragraph"),
            Block::Heading { node_id, .. } => (*node_id, "heading"),
            Block::List { node_id, .. } => (*node_id, "list"),
            Block::Table { node_id, .. } => (*node_id, "table"),
            Block::Figure { node_id, .. } => (*node_id, "figure"),
            Block::PageBreak { node_id, .. } => (*node_id, "page_break"),
        };
        if node_id == owner {
            return Some((node_id, block_type, block.classes()));
        }
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. }
                if inline_tree_contains_owner(children, owner) =>
            {
                return Some((node_id, block_type, block.classes()));
            }
            Block::List { items, .. } => {
                if items.iter().any(|item| item.node_id == owner) {
                    return Some((node_id, block_type, block.classes()));
                }
                for nested in items.iter().rev().flat_map(|item| item.blocks.iter().rev()) {
                    pending.push(nested);
                }
            }
            Block::Table { head, body, .. } => {
                if head.iter().chain(body).any(|row| row.node_id == owner) {
                    return Some((node_id, block_type, block.classes()));
                }
                for nested in body
                    .iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev())
                {
                    pending.push(nested);
                }
            }
            Block::Figure { caption, .. } => {
                pending.extend(caption.iter().rev());
            }
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    None
}

fn inline_tree_contains_owner(inlines: &[Inline], owner: NodeId) -> bool {
    let mut pending: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = pending.pop() {
        let (node_id, children) = match inline {
            Inline::Text { node_id, .. }
            | Inline::Anchor { node_id, .. }
            | Inline::Reference { node_id, .. }
            | Inline::FootnoteReference { node_id, .. }
            | Inline::SoftBreak { node_id, .. }
            | Inline::HardBreak { node_id, .. } => (*node_id, None),
            Inline::Emphasis {
                node_id, children, ..
            }
            | Inline::Strong {
                node_id, children, ..
            }
            | Inline::Link {
                node_id, children, ..
            } => (*node_id, Some(children.as_slice())),
        };
        if node_id == owner {
            return true;
        }
        if let Some(children) = children {
            pending.extend(children.iter().rev());
        }
    }
    false
}

fn parsed_shape_owners(
    document: &Document,
    requested: TextSpan,
) -> Result<(NodeId, NodeId, TextSpan, bool), PackageShapeTextError> {
    let mut matched = None;
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph {
                node_id: style_owner,
                children,
                ..
            }
            | Block::Heading {
                node_id: style_owner,
                children,
                ..
            } => {
                let mut inlines: Vec<&Inline> = children.iter().rev().collect();
                while let Some(inline) = inlines.pop() {
                    match inline {
                        Inline::Text {
                            node_id: site_owner,
                            text_span,
                            ..
                        } if text_span_contains(*text_span, requested) => {
                            if matched.is_some() {
                                return Err(PackageShapeTextError::AmbiguousParsedSpan);
                            }
                            matched = Some((
                                *site_owner,
                                *style_owner,
                                *text_span,
                                inline_logical_site_count(children) == 1,
                            ));
                        }
                        Inline::Emphasis { children, .. }
                        | Inline::Strong { children, .. }
                        | Inline::Link { children, .. } => {
                            inlines.extend(children.iter().rev());
                        }
                        Inline::Text { .. }
                        | Inline::Anchor { .. }
                        | Inline::Reference { .. }
                        | Inline::FootnoteReference { .. }
                        | Inline::SoftBreak { .. }
                        | Inline::HardBreak { .. } => {}
                    }
                }
            }
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    matched.ok_or(PackageShapeTextError::UnownedParsedSpan)
}

fn inline_logical_site_count(inlines: &[Inline]) -> usize {
    let mut count = 0usize;
    let mut pending: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = pending.pop() {
        match inline {
            Inline::Text { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {
                if count == 1 {
                    return 2;
                }
                count = 1;
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => pending.extend(children.iter().rev()),
            Inline::Anchor { .. } => {}
        }
    }
    count
}

fn paragraph_inline_children(document: &Document, owner: NodeId) -> Option<&[Inline]> {
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph {
                node_id, children, ..
            }
            | Block::Heading {
                node_id, children, ..
            } if *node_id == owner => return Some(children),
            Block::Paragraph { .. } | Block::Heading { .. } => {}
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    None
}

fn collect_shape_text_site_identities(
    inlines: &[Inline],
    output: &mut Vec<PackageParagraphTextSite>,
) {
    for inline in inlines {
        match inline {
            Inline::Text { text_span, .. } => {
                output.push(PackageParagraphTextSite::Parsed(*text_span));
            }
            Inline::Reference {
                node_id, format, ..
            } => {
                let generation_kind = match format {
                    ReferenceFormat::Page => GenerationKind::PageReference,
                    ReferenceFormat::Text | ReferenceFormat::Number => GenerationKind::Counter,
                };
                output.push(PackageParagraphTextSite::Generated(
                    GeneratedBufferKey::new(*node_id, generation_kind, 0),
                ));
            }
            Inline::FootnoteReference { node_id, .. } => {
                output.push(PackageParagraphTextSite::Generated(
                    GeneratedBufferKey::new(*node_id, GenerationKind::FootnoteMarker, 0),
                ));
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_shape_text_site_identities(children, output);
            }
            Inline::Anchor { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {}
        }
    }
}

fn generated_inline_site_is_standalone(document: &Document, owner: NodeId) -> bool {
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                if inline_tree_contains_owner(children, owner) {
                    return inline_logical_site_count(children) == 1;
                }
            }
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    false
}

fn text_span_contains(container: TextSpan, requested: TextSpan) -> bool {
    container.text_id() == requested.text_id()
        && container.start_byte().get() <= requested.start_byte().get()
        && requested.end_byte().get() <= container.end_byte().get()
}

fn shape_style_owner(index: &ValidatedDocumentNodeIndex, site_owner: NodeId) -> Option<NodeId> {
    let site_path = index.node_path(site_owner)?;
    if index.node_kind(site_owner) == Some(DocumentNodeKind::FootnoteDefinition) {
        return index
            .nodes()
            .filter(|(candidate, kind)| {
                matches!(
                    kind,
                    DocumentNodeKind::Paragraph | DocumentNodeKind::Heading
                ) && index.node_path(*candidate).is_some_and(|path| {
                    path.starts_with(site_path) && styleable_block_produces_text(index, path)
                })
            })
            .min_by(|(left, _), (right, _)| index.node_path(*left).cmp(&index.node_path(*right)))
            .map(|(owner, _)| owner);
    }
    index
        .nodes()
        .filter(|(candidate, kind)| {
            matches!(
                kind,
                DocumentNodeKind::Paragraph
                    | DocumentNodeKind::Heading
                    | DocumentNodeKind::List
                    | DocumentNodeKind::Table
                    | DocumentNodeKind::Figure
                    | DocumentNodeKind::PageBreak
            ) && index
                .node_path(*candidate)
                .is_some_and(|path| site_path.starts_with(path))
        })
        .max_by_key(|(candidate, _)| index.node_path(*candidate).map(<[u32]>::len))
        .map(|(owner, _)| owner)
}

fn styleable_block_produces_text(index: &ValidatedDocumentNodeIndex, block_path: &[u32]) -> bool {
    index.nodes().any(|(candidate, kind)| {
        matches!(
            kind,
            DocumentNodeKind::Text
                | DocumentNodeKind::Reference
                | DocumentNodeKind::FootnoteReference
                | DocumentNodeKind::SoftBreak
        ) && index
            .node_path(candidate)
            .is_some_and(|path| path.starts_with(block_path) && path.len() > block_path.len())
    })
}

#[cfg(test)]
fn contains_include_directive(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'@' {
            let mut keyword = index + 1;
            while keyword < bytes.len() && bytes[keyword].is_ascii_whitespace() {
                keyword += 1;
            }
            let Some(end) = keyword.checked_add(b"include".len()) else {
                return false;
            };
            let keyword_boundary = match bytes.get(end) {
                Some(byte) => !byte.is_ascii_alphanumeric() && *byte != b'_',
                None => true,
            };
            if bytes.get(keyword..end) == Some(b"include") && keyword_boundary {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn validate_package_limits(
    package: &ParsedPackage,
    policy: &PackageValidationPolicy<'_>,
) -> Result<u64, PackageValidationError> {
    let limits = policy.limits.get();
    let include_count = package
        .sources
        .records()
        .len()
        .checked_sub(1)
        .ok_or(PackageValidationError::MissingEntrySource)?;
    if include_count > limits.max_include_files as usize {
        return Err(PackageValidationError::IncludeFileLimit);
    }
    let mut input_bytes = 0u64;
    for source in package.sources.records() {
        if source.utf8_byte_length() > limits.max_source_bytes {
            return Err(PackageValidationError::SourceByteLimit);
        }
        input_bytes = input_bytes
            .checked_add(u64::from(source.utf8_byte_length()))
            .ok_or(PackageValidationError::InputByteLimit)?;
    }
    if input_bytes > limits.max_input_bytes {
        return Err(PackageValidationError::InputByteLimit);
    }
    let style_rule_count = u64::try_from(package.style_sheet.rules.len())
        .map_err(|_| PackageValidationError::StyleRuleLimit)?;
    if style_rule_count > limits.max_style_rules {
        return Err(PackageValidationError::StyleRuleLimit);
    }
    let mut text_bytes = 0u64;
    for buffer in package.text_store.buffers() {
        if buffer.byte_len() > limits.max_text_buffer_bytes {
            return Err(PackageValidationError::TextBufferByteLimit);
        }
        text_bytes = text_bytes
            .checked_add(u64::from(buffer.byte_len()))
            .ok_or(PackageValidationError::TextByteLimit)?;
    }
    if text_bytes > limits.max_text_bytes {
        return Err(PackageValidationError::TextByteLimit);
    }
    let declaration_count = package
        .style_sheet
        .rules
        .iter()
        .try_fold(0u64, |count, rule| {
            let declarations = u64::try_from(rule.declarations.len()).ok()?;
            count.checked_add(declarations)
        })
        .ok_or(PackageValidationError::AstNodeLimit)?;
    let non_document_ast_nodes = declaration_count
        .checked_mul(2)
        .ok_or(PackageValidationError::AstNodeLimit)?;
    if non_document_ast_nodes > limits.max_ast_nodes {
        return Err(PackageValidationError::AstNodeLimit);
    }
    Ok(non_document_ast_nodes)
}

#[derive(Clone, Copy)]
enum AstPrecheckNode<'a> {
    Document(&'a Document),
    Block(&'a Block),
    Inline(&'a Inline),
    Footnote(&'a FootnoteDefinition),
    ListItem(&'a ListItem),
    TableRow(&'a TableRow),
    TableCell(&'a TableCell),
}

fn push_ast_precheck_node<'a>(
    stack: &mut Vec<(AstPrecheckNode<'a>, u32)>,
    observed_nodes: &mut u64,
    limits: &ValidatedResourceLimits,
    node: AstPrecheckNode<'a>,
    depth: u32,
) -> Result<(), PackageValidationError> {
    if depth > limits.get().max_ast_nesting_depth {
        return Err(PackageValidationError::AstNestingDepthLimit);
    }
    let next_observed = observed_nodes
        .checked_add(1)
        .ok_or(PackageValidationError::AstNodeLimit)?;
    if next_observed > limits.get().max_ast_nodes {
        return Err(PackageValidationError::AstNodeLimit);
    }
    *observed_nodes = next_observed;
    stack.push((node, depth));
    Ok(())
}

/// Performs the depth and node-count checks iteratively before any recursive
/// validation, indexing, or fingerprint traversal can observe the document.
fn validate_document_ast_limits(
    document: &Document,
    limits: &ValidatedResourceLimits,
    non_document_ast_nodes: u64,
) -> Result<(), PackageValidationError> {
    let mut observed_nodes = non_document_ast_nodes;
    let mut stack = Vec::new();
    push_ast_precheck_node(
        &mut stack,
        &mut observed_nodes,
        limits,
        AstPrecheckNode::Document(document),
        1,
    )?;

    while let Some((node, depth)) = stack.pop() {
        let child_depth = depth
            .checked_add(1)
            .ok_or(PackageValidationError::AstNestingDepthLimit)?;
        let mut push = |node| {
            push_ast_precheck_node(&mut stack, &mut observed_nodes, limits, node, child_depth)
        };
        match node {
            AstPrecheckNode::Document(document) => {
                for footnote in document.footnotes.iter().rev() {
                    push(AstPrecheckNode::Footnote(footnote))?;
                }
                for block in document.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
            AstPrecheckNode::Block(block) => match block {
                Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                    for inline in children.iter().rev() {
                        push(AstPrecheckNode::Inline(inline))?;
                    }
                }
                Block::List { items, .. } => {
                    for item in items.iter().rev() {
                        push(AstPrecheckNode::ListItem(item))?;
                    }
                }
                Block::Table { head, body, .. } => {
                    for row in body.iter().rev() {
                        push(AstPrecheckNode::TableRow(row))?;
                    }
                    for row in head.iter().rev() {
                        push(AstPrecheckNode::TableRow(row))?;
                    }
                }
                Block::Figure { caption, .. } => {
                    for block in caption.iter().rev() {
                        push(AstPrecheckNode::Block(block))?;
                    }
                }
                Block::PageBreak { .. } => {}
            },
            AstPrecheckNode::Inline(inline) => match inline {
                Inline::Emphasis { children, .. }
                | Inline::Strong { children, .. }
                | Inline::Link { children, .. } => {
                    for inline in children.iter().rev() {
                        push(AstPrecheckNode::Inline(inline))?;
                    }
                }
                Inline::Text { .. }
                | Inline::Anchor { .. }
                | Inline::Reference { .. }
                | Inline::FootnoteReference { .. }
                | Inline::SoftBreak { .. }
                | Inline::HardBreak { .. } => {}
            },
            AstPrecheckNode::Footnote(footnote) => {
                for block in footnote.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
            AstPrecheckNode::ListItem(item) => {
                for block in item.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
            AstPrecheckNode::TableRow(row) => {
                for cell in row.cells.iter().rev() {
                    push(AstPrecheckNode::TableCell(cell))?;
                }
            }
            AstPrecheckNode::TableCell(cell) => {
                for block in cell.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
        }
    }
    Ok(())
}

/// Bounds the otherwise potentially quadratic parent-chain walks performed by
/// style validation and cascade. Duplicate, unknown, and cyclic graphs are
/// deliberately left to `StyleSheet::validate` so their specific errors win.
fn validate_style_inheritance_depth(
    style_sheet: &StyleSheet,
    limits: &ValidatedResourceLimits,
) -> Result<(), PackageValidationError> {
    let mut by_id = BTreeMap::new();
    for (index, rule) in style_sheet.rules.iter().enumerate() {
        if by_id.insert(&rule.style_id, index).is_some() {
            return Ok(());
        }
    }

    let mut state = vec![0u8; style_sheet.rules.len()];
    let mut depths = vec![0u32; style_sheet.rules.len()];
    for start in 0..style_sheet.rules.len() {
        if state[start] == 2 {
            continue;
        }
        let mut path = Vec::new();
        let mut current = Some(start);
        let base_depth = loop {
            let Some(index) = current else { break 0 };
            match state[index] {
                0 => {
                    state[index] = 1;
                    path.push(index);
                    current = match style_sheet.rules[index].extends.as_ref() {
                        Some(parent) => match by_id.get(parent) {
                            Some(parent_index) => Some(*parent_index),
                            None => return Ok(()),
                        },
                        None => None,
                    };
                }
                1 => return Ok(()),
                2 => break depths[index],
                _ => unreachable!("private style traversal state"),
            }
        };

        let mut depth = base_depth;
        for index in path.into_iter().rev() {
            depth = depth
                .checked_add(1)
                .ok_or(PackageValidationError::AstNestingDepthLimit)?;
            if depth > limits.get().max_ast_nesting_depth {
                return Err(PackageValidationError::AstNestingDepthLimit);
            }
            depths[index] = depth;
            state[index] = 2;
        }
    }
    Ok(())
}

fn validate_source_span(
    package: &ParsedPackage,
    span: SourceSpan,
) -> Result<&typaxis_text::SourceRecord, PackageValidationError> {
    let source = package
        .sources
        .get(span.source_id())
        .ok_or(PackageValidationError::UnknownSource)?;
    if span.end_byte().get() > source.utf8_byte_length() {
        return Err(PackageValidationError::SourceSpanOutOfBounds);
    }
    let start = span.start_byte().get() as usize;
    let end = span.end_byte().get() as usize;
    if !source.utf8().is_char_boundary(start) || !source.utf8().is_char_boundary(end) {
        return Err(PackageValidationError::SourceSpanNotUtf8Boundary);
    }
    Ok(source)
}

fn validate_text_span(
    package: &ParsedPackage,
    span: TextSpan,
) -> Result<(), PackageValidationError> {
    let buffer = package
        .text_store
        .get(span.text_id())
        .ok_or(PackageValidationError::UnknownTextBuffer)?;
    if span.end_byte().get() > buffer.byte_len() {
        return Err(PackageValidationError::TextSpanOutOfBounds);
    }
    if !buffer.is_boundary(span.start_byte()) || !buffer.is_boundary(span.end_byte()) {
        return Err(PackageValidationError::TextSpanNotUtf8Boundary);
    }
    Ok(())
}

fn validate_resource_catalog(
    resources: &ResourceCatalog,
) -> Result<BTreeSet<ImageResourceId>, PackageValidationError> {
    let mut font_ids = BTreeSet::<FontFaceId>::new();
    let mut families = BTreeSet::new();
    for (index, font) in resources.font_faces.iter().enumerate() {
        if font.font_face_id.get()
            != u32::try_from(index).map_err(|_| PackageValidationError::NonCanonicalFontFaceId)?
        {
            return Err(PackageValidationError::NonCanonicalFontFaceId);
        }
        if !font_ids.insert(font.font_face_id) {
            return Err(PackageValidationError::DuplicateFontFaceId);
        }
        if font.family.trim().is_empty() || font.family.chars().any(char::is_control) {
            return Err(PackageValidationError::InvalidFontFamily);
        }
        if !families.insert(font.family.as_str()) {
            return Err(PackageValidationError::DuplicateFontFamily);
        }
    }
    let mut image_ids = BTreeSet::new();
    for (index, image) in resources.images.iter().enumerate() {
        if image.image_id.get()
            != u32::try_from(index).map_err(|_| PackageValidationError::NonCanonicalImageId)?
        {
            return Err(PackageValidationError::NonCanonicalImageId);
        }
        if !image_ids.insert(image.image_id) {
            return Err(PackageValidationError::DuplicateImageId);
        }
    }
    Ok(image_ids)
}

struct DocumentValidator<'a> {
    package: &'a ParsedPackage,
    known_images: &'a BTreeSet<ImageResourceId>,
    node_ids: BTreeSet<NodeId>,
    anchors: BTreeSet<AnchorId>,
    footnote_ids: BTreeSet<FootnoteId>,
    internal_targets: Vec<AnchorId>,
    footnote_targets: Vec<FootnoteId>,
    image_targets: Vec<ImageResourceId>,
    policy: &'a PackageValidationPolicy<'a>,
    next_node_id: u32,
    non_document_ast_nodes: u64,
}

fn validate_document(
    package: &ParsedPackage,
    known_images: &BTreeSet<ImageResourceId>,
    policy: &PackageValidationPolicy<'_>,
    non_document_ast_nodes: u64,
) -> Result<(), PackageValidationError> {
    let mut validator = DocumentValidator {
        package,
        known_images,
        node_ids: BTreeSet::new(),
        anchors: BTreeSet::new(),
        footnote_ids: BTreeSet::new(),
        internal_targets: vec![],
        footnote_targets: vec![],
        image_targets: vec![],
        policy,
        next_node_id: 0,
        non_document_ast_nodes,
    };
    validator.node(package.document.node_id)?;
    for block in &package.document.blocks {
        validator.block(block)?;
    }
    let mut previous_footnote: Option<&FootnoteId> = None;
    for footnote in &package.document.footnotes {
        if previous_footnote.is_some_and(|previous| previous >= &footnote.footnote_id) {
            return Err(PackageValidationError::NonCanonicalFootnoteOrder);
        }
        validator.footnote(footnote)?;
        previous_footnote = Some(&footnote.footnote_id);
    }
    if validator
        .internal_targets
        .iter()
        .any(|target| !validator.anchors.contains(target))
    {
        return Err(PackageValidationError::UnknownInternalTarget);
    }
    if validator
        .footnote_targets
        .iter()
        .any(|target| !validator.footnote_ids.contains(target))
    {
        return Err(PackageValidationError::UnknownFootnoteTarget);
    }
    if validator
        .image_targets
        .iter()
        .any(|target| !validator.known_images.contains(target))
    {
        return Err(PackageValidationError::UnknownImageTarget);
    }
    Ok(())
}

impl DocumentValidator<'_> {
    fn node(&mut self, node_id: NodeId) -> Result<(), PackageValidationError> {
        if !self.node_ids.insert(node_id) {
            return Err(PackageValidationError::DuplicateNodeId);
        }
        if node_id.get() != self.next_node_id {
            return Err(PackageValidationError::NonCanonicalNodeId);
        }
        let total_after_insert = self
            .non_document_ast_nodes
            .checked_add(u64::from(self.next_node_id))
            .and_then(|value| value.checked_add(1))
            .ok_or(PackageValidationError::AstNodeLimit)?;
        if total_after_insert > self.policy.limits.get().max_ast_nodes {
            return Err(PackageValidationError::AstNodeLimit);
        }
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .ok_or(PackageValidationError::AstNodeLimit)?;
        Ok(())
    }

    fn source_node(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
    ) -> Result<(), PackageValidationError> {
        self.node(node_id)?;
        validate_source_span(self.package, span)?;
        Ok(())
    }

    fn classes(&self, classes: &[String]) -> Result<(), PackageValidationError> {
        let mut previous: Option<&str> = None;
        for class in classes {
            if !is_style_identifier(class) {
                return Err(PackageValidationError::InvalidBlockClass);
            }
            if previous == Some(class) {
                return Err(PackageValidationError::DuplicateBlockClass);
            }
            if previous.is_some_and(|value| value > class.as_str()) {
                return Err(PackageValidationError::NonCanonicalBlockClasses);
            }
            previous = Some(class);
        }
        Ok(())
    }

    fn anchor(&mut self, anchor_id: &AnchorId) -> Result<(), PackageValidationError> {
        if !self.anchors.insert(anchor_id.clone()) {
            return Err(PackageValidationError::DuplicateAnchorId);
        }
        Ok(())
    }

    fn block(&mut self, block: &Block) -> Result<(), PackageValidationError> {
        match block {
            Block::Paragraph {
                node_id,
                span,
                classes,
                children,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                self.inlines(children)
            }
            Block::Heading {
                node_id,
                span,
                classes,
                anchor_id,
                children,
                ..
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                if let Some(anchor_id) = anchor_id {
                    self.anchor(anchor_id)?;
                }
                self.inlines(children)
            }
            Block::List {
                node_id,
                span,
                classes,
                ordered,
                start,
                items,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                // The parser resolves an omitted ordered-list source value to
                // `Some(1)` before constructing the canonical package.
                if (*ordered && start.map_or(true, |value| value == 0))
                    || (!*ordered && start.is_some())
                {
                    return Err(PackageValidationError::InvalidListStart);
                }
                if items.is_empty() {
                    return Err(PackageValidationError::EmptyListItems);
                }
                if *ordered {
                    let last_offset = u32::try_from(items.len() - 1)
                        .map_err(|_| PackageValidationError::ListMarkerOverflow)?;
                    start
                        .and_then(|start| start.checked_add(last_offset))
                        .ok_or(PackageValidationError::ListMarkerOverflow)?;
                }
                for item in items {
                    self.list_item(item)?;
                }
                Ok(())
            }
            Block::Table {
                node_id,
                span,
                classes,
                columns,
                head,
                body,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                if columns.is_empty() {
                    return Err(PackageValidationError::EmptyTableColumns);
                }
                if head.is_empty() && body.is_empty() {
                    return Err(PackageValidationError::EmptyTableRows);
                }
                validate_table_grid(columns.len(), head, body)?;
                for row in head.iter().chain(body) {
                    self.table_row(row)?;
                }
                Ok(())
            }
            Block::Figure {
                node_id,
                span,
                classes,
                image_id,
                caption,
                ..
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                self.image_targets.push(*image_id);
                for block in caption {
                    self.block(block)?;
                }
                Ok(())
            }
            Block::PageBreak {
                node_id,
                span,
                classes,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)
            }
        }
    }

    fn inlines(&mut self, inlines: &[Inline]) -> Result<(), PackageValidationError> {
        for inline in inlines {
            self.inline(inline)?;
        }
        Ok(())
    }

    fn inline(&mut self, inline: &Inline) -> Result<(), PackageValidationError> {
        match inline {
            Inline::Text {
                node_id,
                span,
                text_span,
            } => {
                self.source_node(*node_id, *span)?;
                validate_text_span(self.package, *text_span)
            }
            Inline::Emphasis {
                node_id,
                span,
                children,
            }
            | Inline::Strong {
                node_id,
                span,
                children,
            } => {
                self.source_node(*node_id, *span)?;
                self.inlines(children)
            }
            Inline::Link {
                node_id,
                span,
                target,
                children,
            } => {
                self.source_node(*node_id, *span)?;
                match target {
                    LinkTarget::Internal(target) => self.internal_targets.push(target.clone()),
                    LinkTarget::Uri(uri) => {
                        let schemes: Vec<&str> = self
                            .policy
                            .allowed_uri_schemes
                            .iter()
                            .map(String::as_str)
                            .collect();
                        uri.validate_policy(
                            &schemes,
                            self.policy.limits.get().max_uri_bytes as usize,
                        )
                        .map_err(PackageValidationError::InvalidUri)?;
                    }
                }
                self.inlines(children)
            }
            Inline::Anchor {
                node_id,
                span,
                anchor_id,
            } => {
                self.source_node(*node_id, *span)?;
                self.anchor(anchor_id)
            }
            Inline::Reference {
                node_id,
                span,
                target,
                ..
            } => {
                self.source_node(*node_id, *span)?;
                self.internal_targets.push(target.clone());
                Ok(())
            }
            Inline::FootnoteReference {
                node_id,
                span,
                footnote_id,
            } => {
                self.source_node(*node_id, *span)?;
                self.footnote_targets.push(footnote_id.clone());
                Ok(())
            }
            Inline::SoftBreak { node_id, span } | Inline::HardBreak { node_id, span } => {
                self.source_node(*node_id, *span)
            }
        }
    }

    fn list_item(&mut self, item: &ListItem) -> Result<(), PackageValidationError> {
        self.source_node(item.node_id, item.span)?;
        for block in &item.blocks {
            self.block(block)?;
        }
        Ok(())
    }

    fn table_row(&mut self, row: &TableRow) -> Result<(), PackageValidationError> {
        self.source_node(row.node_id, row.span)?;
        for cell in &row.cells {
            self.table_cell(cell)?;
        }
        Ok(())
    }

    fn table_cell(&mut self, cell: &TableCell) -> Result<(), PackageValidationError> {
        self.source_node(cell.node_id, cell.span)?;
        for block in &cell.blocks {
            self.block(block)?;
        }
        Ok(())
    }

    fn footnote(&mut self, footnote: &FootnoteDefinition) -> Result<(), PackageValidationError> {
        if !self.footnote_ids.insert(footnote.footnote_id.clone()) {
            return Err(PackageValidationError::DuplicateFootnoteId);
        }
        self.source_node(footnote.node_id, footnote.span)?;
        for block in &footnote.blocks {
            self.block(block)?;
        }
        Ok(())
    }
}

fn validate_table_grid(
    column_count: usize,
    head: &[TableRow],
    body: &[TableRow],
) -> Result<(), PackageValidationError> {
    let row_count = head
        .len()
        .checked_add(body.len())
        .ok_or(PackageValidationError::InvalidTableGrid)?;
    let mut occupied_rows = vec![0usize; column_count];
    for (row_index, row) in head.iter().chain(body).enumerate() {
        for cell in &row.cells {
            let column_index = occupied_rows
                .iter()
                .position(|remaining| *remaining == 0)
                .ok_or(PackageValidationError::InvalidTableGrid)?;
            let colspan = usize::from(cell.colspan.get());
            let rowspan = usize::from(cell.rowspan.get());
            let column_end = column_index
                .checked_add(colspan)
                .ok_or(PackageValidationError::InvalidTableGrid)?;
            let row_end = row_index
                .checked_add(rowspan)
                .ok_or(PackageValidationError::InvalidTableGrid)?;
            if column_end > column_count
                || row_end > row_count
                || occupied_rows[column_index..column_end]
                    .iter()
                    .any(|remaining| *remaining != 0)
            {
                return Err(PackageValidationError::InvalidTableGrid);
            }
            if row_index < head.len() && row_end > head.len() {
                return Err(PackageValidationError::TableHeadBodyCross);
            }
            occupied_rows[column_index..column_end].fill(rowspan);
        }
        if occupied_rows.contains(&0) {
            return Err(PackageValidationError::InvalidTableGrid);
        }
        for remaining in &mut occupied_rows {
            *remaining -= 1;
        }
    }
    if occupied_rows.iter().any(|remaining| *remaining != 0) {
        return Err(PackageValidationError::InvalidTableGrid);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseOutcome {
    Parsed {
        package: Box<ValidatedParsedPackage>,
        diagnostics: Vec<AdvisoryDiagnostic>,
    },
    Failed {
        failure: ParseFailure,
    },
}
mod parser_seal {
    pub trait Sealed {}
}

/// Trusted parsers are implemented inside this crate; downstream code cannot
/// implement the trait and inject a caller-built AST into `ParseOutcome`.
pub trait Parser: parser_seal::Sealed {
    fn parse(&self, source: &SourceFile, policy: &PackageValidationPolicy<'_>) -> ParseOutcome;
}

/// Small source-driven parser used by the reference workspace to exercise
/// downstream trust boundaries. It accepts only empty lines, `paragraph`,
/// `font:<family>:<portable-path>`, `anchor:<id>`, `reference:<id>`,
/// `soft_break`, `hard_break`, `text:<utf8>`, and
/// `inlines:text=<utf8>|reference=<id>|anchor=<id>` records. The resulting
/// AST, node IDs, spans, text maps, style/resource tables, and default page
/// master are all derived inside this crate; callers never supply a
/// `ParsedPackage`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReferenceParser;
impl ReferenceParser {
    pub const fn new() -> Self {
        Self
    }
}
impl parser_seal::Sealed for ReferenceParser {}
impl Parser for ReferenceParser {
    fn parse(&self, source: &SourceFile, policy: &PackageValidationPolicy<'_>) -> ParseOutcome {
        let package = match parse_reference_entry(source) {
            Ok(package) => package,
            Err(message) => return reference_parse_failure(message),
        };
        let include_graph = match ValidatedIncludeGraph::entry_only(&package.sources, policy.limits)
        {
            Ok(include_graph) => include_graph,
            Err(_) => return reference_parse_failure("entry include closure was rejected"),
        };
        match ValidatedParsedPackage::new_resolved(package, policy, &include_graph) {
            Ok(package) => ParseOutcome::Parsed {
                package: Box::new(package),
                diagnostics: vec![],
            },
            Err(_) => reference_parse_failure("reference source failed package validation"),
        }
    }
}

fn parse_reference_entry(source: &SourceFile) -> Result<ParsedPackage, &'static str> {
    if source.source_id != SourceId::new(0) {
        return Err("entry SourceId must be zero");
    }
    let source_record =
        SourceRecord::new(source.source_id, source.uri.clone(), source.text.clone())
            .map_err(|_| "entry source is too large")?;
    let sources = SourceCatalog::new(vec![source_record])
        .map_err(|_| "entry source catalog is not canonical")?;
    let mut blocks = Vec::new();
    let mut text_buffers = Vec::new();
    let mut font_faces = Vec::new();
    let mut next_node = 1u32;
    let mut start = 0usize;
    for raw_line in source.text.split_inclusive('\n') {
        let without_lf = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        let end = start
            .checked_add(line.len())
            .ok_or("source span overflow")?;
        if !line.is_empty() {
            if let Some(declaration) = line.strip_prefix("font:") {
                let (family, uri) = declaration
                    .split_once(':')
                    .ok_or("font record must contain family and portable path")?;
                if family.trim().is_empty() || family.chars().any(char::is_control) {
                    return Err("font family is invalid");
                }
                font_faces.push(FontFaceDeclaration {
                    font_face_id: FontFaceId::new(
                        u32::try_from(font_faces.len()).map_err(|_| "font ID overflow")?,
                    ),
                    family: family.to_owned(),
                    uri: PortablePath::new(uri).map_err(|_| "font path is invalid")?,
                    face_index: 0,
                    expected_sha256: None,
                });
                start = start
                    .checked_add(raw_line.len())
                    .ok_or("source span overflow")?;
                continue;
            }
            let start_byte = u32::try_from(start).map_err(|_| "source span overflow")?;
            let end_byte = u32::try_from(end).map_err(|_| "source span overflow")?;
            let span = SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(start_byte),
                Utf8ByteOffset::new(end_byte),
            )
            .ok_or("source span is invalid")?;
            let paragraph_id = NodeId::new(next_node);
            next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
            let children = if let Some(sequence) = line.strip_prefix("inlines:") {
                parse_reference_inline_sequence(sequence, start, &mut next_node, &mut text_buffers)?
            } else if let Some(anchor) = line.strip_prefix("anchor:") {
                let anchor_id = AnchorId::new(anchor).map_err(|_| "anchor ID is invalid")?;
                let anchor_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                vec![Inline::Anchor {
                    node_id: anchor_node,
                    span,
                    anchor_id,
                }]
            } else if let Some(target) = line.strip_prefix("reference:") {
                let target = AnchorId::new(target).map_err(|_| "reference target is invalid")?;
                let reference_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                vec![Inline::Reference {
                    node_id: reference_node,
                    span,
                    target,
                    format: ReferenceFormat::Page,
                }]
            } else if let Some(text) = line.strip_prefix("text:") {
                if text.is_empty() {
                    return Err("text record must not be empty");
                }
                let text_start = start
                    .checked_add("text:".len())
                    .ok_or("source span overflow")?;
                let text_end = text_start
                    .checked_add(text.len())
                    .ok_or("source span overflow")?;
                let source_start = u32::try_from(text_start).map_err(|_| "source span overflow")?;
                let source_end = u32::try_from(text_end).map_err(|_| "source span overflow")?;
                let text_len = u32::try_from(text.len()).map_err(|_| "text buffer overflow")?;
                let text_id = TextBufferId::new(
                    u32::try_from(text_buffers.len()).map_err(|_| "text buffer ID overflow")?,
                );
                let text_source_span = SourceSpan::new(
                    SourceId::new(0),
                    Utf8ByteOffset::new(source_start),
                    Utf8ByteOffset::new(source_end),
                )
                .ok_or("text source span is invalid")?;
                let text_range =
                    Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(text_len))
                        .ok_or("text range is invalid")?;
                text_buffers.push(
                    TextBuffer::new(
                        text_id,
                        text.to_owned(),
                        vec![TextMapSegment {
                            text_range,
                            kind: TextMapKind::Identity,
                            source_span: Some(text_source_span),
                        }],
                        text_len,
                    )
                    .map_err(|_| "text buffer was rejected")?,
                );
                let text_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                vec![Inline::Text {
                    node_id: text_node,
                    span: text_source_span,
                    text_span: TextSpan::new(
                        text_id,
                        Utf8ByteOffset::new(0),
                        Utf8ByteOffset::new(text_len),
                    )
                    .ok_or("text span is invalid")?,
                }]
            } else if line == "paragraph" {
                vec![]
            } else if line == "soft_break" || line == "hard_break" {
                let break_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                if line == "soft_break" {
                    vec![Inline::SoftBreak {
                        node_id: break_node,
                        span,
                    }]
                } else {
                    vec![Inline::HardBreak {
                        node_id: break_node,
                        span,
                    }]
                }
            } else {
                return Err("unsupported reference source record");
            };
            blocks.push(Block::Paragraph {
                node_id: paragraph_id,
                span,
                classes: vec![],
                children,
            });
        }
        start = start
            .checked_add(raw_line.len())
            .ok_or("source span overflow")?;
    }
    // The reference grammar has no page/style declarations of its own, so it
    // supplies one deterministic, physically meaningful default: A4 with a
    // 20 mm body margin, 10.5 pt text, and a 17 pt line height. Keep all unit
    // conversion on the canonical rational-PDF-point path.
    let page_width = PositiveLength::new(
        Length::from_rational_pdf_points(210 * 720, 254)
            .map_err(|_| "invalid default page width")?,
    )
    .ok_or("invalid default page width")?;
    let page_height = PositiveLength::new(
        Length::from_rational_pdf_points(297 * 720, 254)
            .map_err(|_| "invalid default page height")?,
    )
    .ok_or("invalid default page height")?;
    let body_margin = Length::from_rational_pdf_points(20 * 720, 254)
        .map_err(|_| "invalid default page margin")?;
    let body_width = PositiveLength::new(
        page_width
            .get()
            .checked_sub(body_margin)
            .and_then(|value| value.checked_sub(body_margin))
            .ok_or("invalid default body width")?,
    )
    .ok_or("invalid default body width")?;
    let body_height = PositiveLength::new(
        page_height
            .get()
            .checked_sub(body_margin)
            .and_then(|value| value.checked_sub(body_margin))
            .ok_or("invalid default body height")?,
    )
    .ok_or("invalid default body height")?;
    let default_font_size =
        Length::from_rational_pdf_points(21, 2).map_err(|_| "invalid default font size")?;
    let default_line_height =
        Length::from_rational_pdf_points(17, 1).map_err(|_| "invalid default line height")?;
    Ok(ParsedPackage {
        sources,
        text_store: TextStore::new(text_buffers).map_err(|_| "text store was rejected")?,
        document: Document {
            node_id: NodeId::new(0),
            blocks,
            footnotes: vec![],
        },
        style_sheet: StyleSheet {
            rules: if font_faces.is_empty() {
                vec![]
            } else {
                vec![StyleRule {
                    style_id: StyleId::new("reference_text")
                        .map_err(|_| "default style ID is invalid")?,
                    extends: None,
                    selector: "paragraph".to_owned(),
                    source_order: 0,
                    declarations: vec![
                        Declaration {
                            name: "font_family".to_owned(),
                            value: StyleValue::FontFamilyList(
                                font_faces.iter().map(|font| font.family.clone()).collect(),
                            ),
                            important: false,
                        },
                        Declaration {
                            name: "font_size".to_owned(),
                            value: StyleValue::Length(default_font_size),
                            important: false,
                        },
                        Declaration {
                            name: "line_height".to_owned(),
                            value: StyleValue::Length(default_line_height),
                            important: false,
                        },
                    ],
                }]
            },
        },
        page_masters: PageMasterSet {
            default_master_id: MasterId::new("default").map_err(|_| "invalid master ID")?,
            masters: vec![PageMaster {
                master_id: MasterId::new("default").map_err(|_| "invalid master ID")?,
                width: page_width,
                height: page_height,
                body: Rect::new(body_margin, body_margin, body_width, body_height),
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: vec![],
        },
        resources: ResourceCatalog {
            font_faces,
            images: vec![],
        },
    })
}

fn parse_reference_inline_sequence(
    sequence: &str,
    line_start: usize,
    next_node: &mut u32,
    text_buffers: &mut Vec<TextBuffer>,
) -> Result<Vec<Inline>, &'static str> {
    if sequence.is_empty() || sequence.ends_with('|') {
        return Err("inline sequence is empty or has an empty final component");
    }
    let prefix_len = "inlines:".len();
    let mut local_start = 0usize;
    let mut children = Vec::new();
    for raw_component in sequence.split_inclusive('|') {
        let component = raw_component.strip_suffix('|').unwrap_or(raw_component);
        if component.is_empty() {
            return Err("inline sequence has an empty component");
        }
        let component_source_start = line_start
            .checked_add(prefix_len)
            .and_then(|value| value.checked_add(local_start))
            .ok_or("source span overflow")?;
        let component_source_end = component_source_start
            .checked_add(component.len())
            .ok_or("source span overflow")?;
        let component_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(
                u32::try_from(component_source_start).map_err(|_| "source span overflow")?,
            ),
            Utf8ByteOffset::new(
                u32::try_from(component_source_end).map_err(|_| "source span overflow")?,
            ),
        )
        .ok_or("inline component span is invalid")?;
        let node_id = NodeId::new(*next_node);
        *next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
        if let Some(text) = component.strip_prefix("text=") {
            if text.is_empty() {
                return Err("inline text component must not be empty");
            }
            let text_source_start = component_source_start
                .checked_add("text=".len())
                .ok_or("source span overflow")?;
            let text_source_end = text_source_start
                .checked_add(text.len())
                .ok_or("source span overflow")?;
            let text_len = u32::try_from(text.len()).map_err(|_| "text buffer overflow")?;
            let text_id = TextBufferId::new(
                u32::try_from(text_buffers.len()).map_err(|_| "text buffer ID overflow")?,
            );
            let source_span = SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(
                    u32::try_from(text_source_start).map_err(|_| "source span overflow")?,
                ),
                Utf8ByteOffset::new(
                    u32::try_from(text_source_end).map_err(|_| "source span overflow")?,
                ),
            )
            .ok_or("inline text source span is invalid")?;
            let text_range =
                Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(text_len))
                    .ok_or("text range is invalid")?;
            text_buffers.push(
                TextBuffer::new(
                    text_id,
                    text.to_owned(),
                    vec![TextMapSegment {
                        text_range,
                        kind: TextMapKind::Identity,
                        source_span: Some(source_span),
                    }],
                    text_len,
                )
                .map_err(|_| "text buffer was rejected")?,
            );
            children.push(Inline::Text {
                node_id,
                span: source_span,
                text_span: TextSpan::new(
                    text_id,
                    Utf8ByteOffset::new(0),
                    Utf8ByteOffset::new(text_len),
                )
                .ok_or("text span is invalid")?,
            });
        } else if let Some(target) = component.strip_prefix("reference=") {
            children.push(Inline::Reference {
                node_id,
                span: component_span,
                target: AnchorId::new(target).map_err(|_| "reference target is invalid")?,
                format: ReferenceFormat::Page,
            });
        } else if let Some(anchor) = component.strip_prefix("anchor=") {
            children.push(Inline::Anchor {
                node_id,
                span: component_span,
                anchor_id: AnchorId::new(anchor).map_err(|_| "anchor ID is invalid")?,
            });
        } else if component == "soft_break" || component == "hard_break" {
            children.push(if component == "soft_break" {
                Inline::SoftBreak {
                    node_id,
                    span: component_span,
                }
            } else {
                Inline::HardBreak {
                    node_id,
                    span: component_span,
                }
            });
        } else {
            return Err("unsupported inline sequence component");
        }
        local_start = local_start
            .checked_add(raw_component.len())
            .ok_or("source span overflow")?;
    }
    Ok(children)
}

fn reference_parse_failure(message: &'static str) -> ParseOutcome {
    let diagnostic = Diagnostic::new(
        DiagnosticCode::new("P1000").expect("static diagnostic code is valid"),
        Severity::Error,
        message,
    )
    .expect("static diagnostic message is nonempty");
    let mut phase = PhaseDiagnostics::new();
    let flow = phase.emit(diagnostic);
    debug_assert_eq!(flow, DiagnosticFlow::Continue);
    ParseOutcome::Failed {
        failure: phase
            .finish_boundary()
            .expect_err("the safe phase boundary contains an error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::num::NonZeroU16;
    use typaxis_core::{
        GeneratedBufferKey, GenerationKind, Length, MasterId, PositiveLength, Rect, ResourceLimits,
        SourceSpan, StyleId, TextBufferId, Utf8ByteOffset, Utf8ByteRange, ValidatedResourceLimits,
    };
    use typaxis_document::{
        ColumnSizing, FontFaceDeclaration, ImageDeclaration, ListItem, TableColumn,
    };
    use typaxis_style::{Declaration, PageMaster, StyleRule};
    use typaxis_text::{
        GeneratedBufferDraft, SourceRecord, TextBuffer, TextMapKind, TextMapSegment,
    };

    fn empty_package(sources: SourceCatalog, text_store: TextStore) -> ParsedPackage {
        let size = PositiveLength::new(Length::from_raw(100).unwrap()).unwrap();
        ParsedPackage {
            sources,
            text_store,
            document: Document {
                node_id: typaxis_core::NodeId::new(0),
                blocks: vec![],
                footnotes: vec![],
            },
            style_sheet: StyleSheet { rules: vec![] },
            page_masters: PageMasterSet {
                default_master_id: MasterId::new("default").unwrap(),
                masters: vec![PageMaster {
                    master_id: MasterId::new("default").unwrap(),
                    width: size,
                    height: size,
                    body: Rect::new(Length::ZERO, Length::ZERO, size, size),
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: vec![],
            },
            resources: ResourceCatalog {
                font_faces: vec![],
                images: vec![],
            },
        }
    }

    fn validate(package: ParsedPackage) -> Result<ValidatedParsedPackage, PackageValidationError> {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = vec![
            "http".to_owned(),
            "https".to_owned(),
            "mailto".to_owned(),
            "tel".to_owned(),
        ];
        ValidatedParsedPackage::new_entry_only(
            package,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        )
    }

    fn validate_with_limits(
        package: ParsedPackage,
        limits: ResourceLimits,
    ) -> Result<ValidatedParsedPackage, PackageValidationError> {
        let limits = ValidatedResourceLimits::new(limits).unwrap();
        let schemes = vec![
            "http".to_owned(),
            "https".to_owned(),
            "mailto".to_owned(),
            "tel".to_owned(),
        ];
        ValidatedParsedPackage::new_entry_only(
            package,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        )
    }

    fn empty_package_with_source() -> ParsedPackage {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            String::new(),
        )
        .unwrap();
        empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![]).unwrap(),
        )
    }

    #[test]
    fn successful_outcome_requires_validated_package() {
        let package = empty_package_with_source();
        let package = validate(package).unwrap();
        let outcome = ParseOutcome::Parsed {
            package: Box::new(package),
            diagnostics: vec![],
        };
        assert!(matches!(outcome, ParseOutcome::Parsed { .. }));
    }

    #[test]
    fn reference_parser_derives_trusted_facts_from_source_records() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "anchor:chapter\ntext:actual".to_owned(),
        };
        let outcome = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = outcome else {
            panic!("reference source must parse");
        };
        assert_eq!(package.package().document.blocks.len(), 2);
        assert_eq!(package.package().text_store.buffers()[0].text(), "actual");
        assert_eq!(
            package
                .document_nodes()
                .anchor_owner(&AnchorId::new("chapter").unwrap()),
            Some(NodeId::new(2))
        );
        let requested = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(1),
            Utf8ByteOffset::new(4),
        )
        .unwrap();
        let receipt = package.bind_parsed_shape_text(requested).unwrap();
        assert_eq!(receipt.source(), PackageShapeTextSource::Parsed(requested));
        assert_eq!(receipt.site_owner(), NodeId::new(4));
        assert_eq!(receipt.style_owner(), NodeId::new(3));
        assert_eq!(receipt.utf8(), "ctu");
        assert_eq!(receipt.reference_fingerprint(), None);
        assert!(!receipt.covers_complete_site());
        assert!(receipt.is_standalone_logical_text());
        assert_eq!(
            receipt.document_fingerprint(),
            package.epoch_identity().document()
        );

        let complete = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(6),
        )
        .unwrap();
        assert!(package
            .bind_parsed_shape_text(complete)
            .unwrap()
            .covers_complete_site());
    }

    #[test]
    fn reference_parser_uses_physical_a4_text_defaults() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "font:Reference:Reference.ttf\ntext:actual".to_owned(),
        };
        let ParseOutcome::Parsed { package, .. } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) else {
            panic!("reference source must parse");
        };
        let master = &package.package().page_masters.masters[0];
        assert_eq!(master.width.get().raw(), 39_011_981);
        assert_eq!(master.height.get().raw(), 55_174_088);
        assert_eq!(master.body.x().raw(), 3_715_427);
        assert_eq!(master.body.y().raw(), 3_715_427);
        assert_eq!(master.body.width().get().raw(), 31_581_127);
        assert_eq!(master.body.height().get().raw(), 47_743_234);

        let computed = package.cascade_style(NodeId::new(2)).unwrap();
        assert_eq!(
            computed.computed().properties().get("font_size"),
            Some(&StyleValue::Length(Length::from_raw(688_128).unwrap()))
        );
        assert_eq!(
            computed.computed().properties().get("line_height"),
            Some(&StyleValue::Length(Length::from_raw(1_114_112).unwrap()))
        );
    }

    #[test]
    fn reference_parser_derives_canonical_adjacent_inline_sites() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "anchor:chapter\ninlines:text=See |reference=chapter|text= now".to_owned(),
        };
        let outcome = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = outcome else {
            panic!("reference source must parse");
        };
        let Block::Paragraph {
            node_id: paragraph, ..
        } = package.package().document.blocks[1]
        else {
            panic!("inline record must derive a paragraph")
        };
        let sites = package.paragraph_shape_text_sites(paragraph).unwrap();
        assert_eq!(sites.len(), 3);
        assert!(matches!(sites[0], PackageParagraphTextSite::Parsed(_)));
        assert!(matches!(
            sites[1],
            PackageParagraphTextSite::Generated(key)
                if key.generation_kind() == GenerationKind::PageReference
        ));
        assert!(matches!(sites[2], PackageParagraphTextSite::Parsed(_)));
    }

    #[test]
    fn parsed_shape_receipt_marks_adjacent_inline_text_as_non_standalone() {
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(2)).unwrap();
        let text_store = TextStore::new(vec![TextBuffer::new(
            TextBufferId::new(0),
            "ab".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Inserted,
                source_span: None,
            }],
            2,
        )
        .unwrap()])
        .unwrap();
        let mut parsed = empty_package(
            SourceCatalog::new(vec![SourceRecord::new(
                SourceId::new(0),
                PortablePath::new("input.tsf").unwrap(),
                String::new(),
            )
            .unwrap()])
            .unwrap(),
            text_store,
        );
        let first = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let second = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(1),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        parsed.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span: source_span,
            classes: vec![],
            children: vec![
                Inline::Text {
                    node_id: NodeId::new(2),
                    span: source_span,
                    text_span: first,
                },
                Inline::Text {
                    node_id: NodeId::new(3),
                    span: source_span,
                    text_span: second,
                },
            ],
        });
        let package = validate(parsed).unwrap();
        let receipt = package.bind_parsed_shape_text(first).unwrap();
        assert!(receipt.covers_complete_site());
        assert!(!receipt.is_standalone_logical_text());
        assert_eq!(
            package.paragraph_shape_text_sites(NodeId::new(1)).unwrap(),
            [
                PackageParagraphTextSite::Parsed(first),
                PackageParagraphTextSite::Parsed(second)
            ]
        );
    }

    #[test]
    fn reference_parser_rejects_non_grammar_source() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "caller-authored AST marker".to_owned(),
        };
        assert!(matches!(
            ReferenceParser::new().parse(
                &source,
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            ),
            ParseOutcome::Failed { .. }
        ));
    }

    #[test]
    fn generated_text_binding_rechecks_the_actual_package_text_limits() {
        let buffer = |text_id| {
            let range =
                Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(16)).unwrap();
            TextBuffer::new(
                TextBufferId::new(text_id),
                "x".repeat(16),
                vec![TextMapSegment {
                    text_range: range,
                    kind: TextMapKind::Inserted,
                    source_span: None,
                }],
                16,
            )
            .unwrap()
        };
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            String::new(),
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![buffer(0), buffer(1)]).unwrap(),
        );
        let package = validate(package).unwrap();

        let limits = |max_text_bytes| {
            ValidatedResourceLimits::new(ResourceLimits {
                max_text_bytes,
                max_text_buffer_bytes: 16,
                max_shaping_context_bytes: 16,
                ..ResourceLimits::default()
            })
            .unwrap()
        };
        let exact = limits(32);
        // Construct the generated store against an unrelated empty parsed
        // store; package binding must still recompute the actual package's
        // parsed-plus-generated totals before accepting it.
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &exact,
            &TextStore::new(vec![]).unwrap(),
        )
        .unwrap();
        assert!(package.bind_generated_text(&generated, &exact).is_ok());

        let below = limits(31);
        assert_eq!(
            package.bind_generated_text(&generated, &below),
            Err(PackageGeneratedTextError::TextTotalLimit)
        );
    }

    #[test]
    fn generated_shape_text_binds_site_style_and_selected_overlay() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![Inline::SoftBreak {
                node_id: NodeId::new(2),
                span,
            }],
        });
        let package = validate(package).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let key = GeneratedBufferKey::new(NodeId::new(2), GenerationKind::Discretionary, 0);
        let generated = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), key, "xy".to_owned()).unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let provenance = generated
            .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(2))
            .unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let receipt = binding.bind_generated_shape_text(provenance).unwrap();
        assert_eq!(
            receipt.source(),
            PackageShapeTextSource::Generated(provenance)
        );
        assert_eq!(receipt.site_owner(), NodeId::new(2));
        assert_eq!(receipt.style_owner(), NodeId::new(1));
        assert_eq!(receipt.utf8(), "xy");
        assert!(receipt.covers_complete_site());
        assert!(!receipt.is_standalone_logical_text());
        assert_eq!(
            receipt.reference_fingerprint(),
            Some(generated.reference_fingerprint())
        );
    }

    #[test]
    fn initial_generated_overlay_materializes_explicit_break_sites() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![
                Inline::SoftBreak {
                    node_id: NodeId::new(2),
                    span,
                },
                Inline::HardBreak {
                    node_id: NodeId::new(3),
                    span,
                },
            ],
        });
        let package = validate(package).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = package.materialize_initial_generated_text(&limits).unwrap();
        assert_eq!(generated.buffers().len(), 2);
        assert!(generated.buffers().iter().all(|buffer| {
            buffer.key().generation_kind() == GenerationKind::Discretionary
                && buffer.utf8().is_empty()
        }));
        package.bind_generated_text(&generated, &limits).unwrap();
    }

    #[test]
    fn footnote_marker_uses_first_text_producing_descendant_style() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.footnotes.push(FootnoteDefinition {
            footnote_id: FootnoteId::new("note").unwrap(),
            node_id: NodeId::new(1),
            span,
            blocks: vec![Block::Paragraph {
                node_id: NodeId::new(2),
                span,
                classes: vec![],
                children: vec![Inline::SoftBreak {
                    node_id: NodeId::new(3),
                    span,
                }],
            }],
        });
        let package = validate(package).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let marker = GeneratedBufferKey::new(NodeId::new(1), GenerationKind::FootnoteMarker, 0);
        let discretionary =
            GeneratedBufferKey::new(NodeId::new(3), GenerationKind::Discretionary, 0);
        let generated = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), marker, "1".to_owned())
                    .unwrap(),
                GeneratedBufferDraft::new(package.document_nodes(), discretionary, " ".to_owned())
                    .unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let provenance = generated
            .provenance(marker, Utf8ByteOffset::new(0), Utf8ByteOffset::new(1))
            .unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let receipt = binding.bind_generated_shape_text(provenance).unwrap();
        assert_eq!(receipt.site_owner(), NodeId::new(1));
        assert_eq!(receipt.style_owner(), NodeId::new(2));
    }

    #[test]
    fn page_selection_is_issued_from_the_package_style_and_owner() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![],
        });
        package.style_sheet.rules.push(StyleRule {
            style_id: StyleId::new("paragraph-page").unwrap(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 0,
            declarations: vec![Declaration {
                name: "page".to_owned(),
                value: StyleValue::Text("chapter".to_owned()),
                important: false,
            }],
        });
        let package = validate(package).unwrap();
        let selection = package.resolve_page_selection(NodeId::new(1)).unwrap();
        assert_eq!(selection.owner(), NodeId::new(1));
        assert_eq!(selection.page_name().map(PageName::as_str), Some("chapter"));
        assert_eq!(
            package.resolve_page_selection(NodeId::new(0)),
            Err(PackageStyleError::UnknownStyleOwner)
        );
        assert_eq!(
            package.resolve_blank_page_selection(),
            Err(PackageStyleError::NonEmptyDocument)
        );
    }

    #[test]
    fn list_item_flow_owner_resolves_its_nearest_styleable_list() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec!["chapter".to_owned()],
            ordered: false,
            start: None,
            items: vec![ListItem {
                node_id: NodeId::new(2),
                span,
                blocks: vec![],
            }],
        });
        package.style_sheet.rules.push(StyleRule {
            style_id: StyleId::new("list-page").unwrap(),
            extends: None,
            selector: "list.chapter".to_owned(),
            source_order: 0,
            declarations: vec![Declaration {
                name: "page".to_owned(),
                value: StyleValue::Text("chapter".to_owned()),
                important: false,
            }],
        });
        let package = validate(package).unwrap();
        let selection = package.resolve_page_selection(NodeId::new(2)).unwrap();
        assert_eq!(selection.owner(), NodeId::new(2));
        assert_eq!(selection.style_owner(), NodeId::new(1));
        assert_eq!(selection.page_name().map(PageName::as_str), Some("chapter"));
    }

    #[test]
    fn package_prechecks_ast_nesting_before_recursive_validation() {
        let nested_package = || {
            let span = SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(0),
            )
            .unwrap();
            let mut inline = Inline::SoftBreak {
                node_id: NodeId::new(4),
                span,
            };
            for node_id in (2..=3).rev() {
                inline = Inline::Strong {
                    node_id: NodeId::new(node_id),
                    span,
                    children: vec![inline],
                };
            }
            let mut package = empty_package_with_source();
            package.document.blocks.push(Block::Paragraph {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                children: vec![inline],
            });
            package
        };

        assert!(validate_with_limits(
            nested_package(),
            ResourceLimits {
                max_ast_nesting_depth: 5,
                ..ResourceLimits::default()
            },
        )
        .is_ok());
        assert_eq!(
            validate_with_limits(
                nested_package(),
                ResourceLimits {
                    max_ast_nesting_depth: 4,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::AstNestingDepthLimit)
        );
    }

    #[test]
    fn package_bounds_style_inheritance_and_preserves_graph_errors() {
        let style_chain = |length: u32| {
            let mut package = empty_package_with_source();
            package.style_sheet.rules = (0..length)
                .map(|index| StyleRule {
                    style_id: typaxis_core::StyleId::new(format!("s{index}")).unwrap(),
                    extends: index
                        .checked_sub(1)
                        .map(|parent| typaxis_core::StyleId::new(format!("s{parent}")).unwrap()),
                    selector: "paragraph".to_owned(),
                    source_order: index,
                    declarations: vec![],
                })
                .collect();
            package
        };

        assert!(validate_with_limits(
            style_chain(4),
            ResourceLimits {
                max_ast_nesting_depth: 4,
                ..ResourceLimits::default()
            },
        )
        .is_ok());
        assert_eq!(
            validate_with_limits(
                style_chain(4),
                ResourceLimits {
                    max_ast_nesting_depth: 3,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::AstNestingDepthLimit)
        );

        let mut unknown = style_chain(1);
        unknown.style_sheet.rules[0].extends = Some(typaxis_core::StyleId::new("missing").unwrap());
        assert_eq!(
            validate_with_limits(
                unknown,
                ResourceLimits {
                    max_ast_nesting_depth: 1,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::InvalidStyle(
                StyleValidationError::UnknownParent
            ))
        );

        let mut cycle = style_chain(2);
        cycle.style_sheet.rules[0].extends = Some(typaxis_core::StyleId::new("s1").unwrap());
        assert_eq!(
            validate_with_limits(
                cycle,
                ResourceLimits {
                    max_ast_nesting_depth: 1,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::InvalidStyle(
                StyleValidationError::InheritanceCycle
            ))
        );
    }

    #[test]
    fn package_rejects_unknown_mapped_source() {
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(7),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "x".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Replacement,
                source_span: Some(source_span),
            }],
            1,
        )
        .unwrap();
        let entry = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            String::new(),
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![entry]).unwrap(),
            TextStore::new(vec![buffer]).unwrap(),
        );
        assert_eq!(
            validate(package),
            Err(PackageValidationError::UnknownSource)
        );
    }

    #[test]
    fn package_rejects_out_of_bounds_source_span() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "x".to_owned(),
        )
        .unwrap();
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(2)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "xx".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Identity,
                source_span: Some(source_span),
            }],
            2,
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![buffer]).unwrap(),
        );
        assert_eq!(
            validate(package),
            Err(PackageValidationError::SourceSpanOutOfBounds)
        );
    }

    #[test]
    fn package_rejects_identity_bytes_that_only_match_in_length() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "a".to_owned(),
        )
        .unwrap();
        let range = Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "b".to_owned(),
            vec![TextMapSegment {
                text_range: range,
                kind: TextMapKind::Identity,
                source_span: Some(source_span),
            }],
            1,
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![buffer]).unwrap(),
        );
        assert_eq!(
            validate(package),
            Err(PackageValidationError::IdentityBytesMismatch)
        );
    }

    #[test]
    fn package_enforces_list_start_and_table_column_semantics() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut unordered = empty_package_with_source();
        unordered.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: false,
            start: Some(1),
            items: vec![],
        });
        assert_eq!(
            validate(unordered),
            Err(PackageValidationError::InvalidListStart)
        );

        let mut ordered = empty_package_with_source();
        ordered.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: true,
            start: None,
            items: vec![],
        });
        assert_eq!(
            validate(ordered),
            Err(PackageValidationError::InvalidListStart)
        );

        let mut zero = empty_package_with_source();
        zero.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: true,
            start: Some(0),
            items: vec![],
        });
        assert_eq!(
            validate(zero),
            Err(PackageValidationError::InvalidListStart)
        );

        let mut empty_list = empty_package_with_source();
        empty_list.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: false,
            start: None,
            items: vec![],
        });
        assert_eq!(
            validate(empty_list),
            Err(PackageValidationError::EmptyListItems)
        );

        let empty_item = |node_id| ListItem {
            node_id: NodeId::new(node_id),
            span,
            blocks: vec![],
        };
        let mut overflowing = empty_package_with_source();
        overflowing.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: true,
            start: Some(u32::MAX),
            items: vec![empty_item(2), empty_item(3)],
        });
        assert_eq!(
            validate(overflowing),
            Err(PackageValidationError::ListMarkerOverflow)
        );

        let mut table = empty_package_with_source();
        table.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![],
            head: vec![],
            body: vec![],
        });
        assert_eq!(
            validate(table),
            Err(PackageValidationError::EmptyTableColumns)
        );

        let width = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
        let mut empty_table = empty_package_with_source();
        empty_table.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![TableColumn {
                sizing: ColumnSizing::Fixed(width),
            }],
            head: vec![],
            body: vec![],
        });
        assert_eq!(
            validate(empty_table),
            Err(PackageValidationError::EmptyTableRows)
        );

        let mut incomplete_grid = empty_package_with_source();
        incomplete_grid.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![TableColumn {
                sizing: ColumnSizing::Fixed(width),
            }],
            head: vec![],
            body: vec![TableRow {
                node_id: NodeId::new(2),
                span,
                cells: vec![],
            }],
        });
        assert_eq!(
            validate(incomplete_grid),
            Err(PackageValidationError::InvalidTableGrid)
        );

        let mut crossing = empty_package_with_source();
        crossing.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![TableColumn {
                sizing: ColumnSizing::Fixed(width),
            }],
            head: vec![TableRow {
                node_id: NodeId::new(2),
                span,
                cells: vec![TableCell {
                    node_id: NodeId::new(3),
                    span,
                    colspan: NonZeroU16::new(1).unwrap(),
                    rowspan: NonZeroU16::new(2).unwrap(),
                    blocks: vec![],
                }],
            }],
            body: vec![TableRow {
                node_id: NodeId::new(4),
                span,
                cells: vec![],
            }],
        });
        assert_eq!(
            validate(crossing),
            Err(PackageValidationError::TableHeadBodyCross)
        );
    }

    #[test]
    fn every_list_item_has_one_canonical_generated_marker() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let item = |node_id| ListItem {
            node_id: NodeId::new(node_id),
            span,
            blocks: vec![],
        };
        let mut package = empty_package_with_source();
        package.document.blocks.extend([
            Block::List {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                ordered: true,
                start: Some(9),
                items: vec![item(2), item(3)],
            },
            Block::List {
                node_id: NodeId::new(4),
                span,
                classes: vec![],
                ordered: false,
                start: None,
                items: vec![item(5)],
            },
        ]);
        let package = validate(package).unwrap();
        let key =
            |owner| GeneratedBufferKey::new(NodeId::new(owner), GenerationKind::ListMarker, 0);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![
                package.materialize_list_marker(key(2)).unwrap(),
                package.materialize_list_marker(key(3)).unwrap(),
                package.materialize_list_marker(key(5)).unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let bytes: Vec<_> = generated
            .buffers()
            .iter()
            .map(|buffer| buffer.utf8())
            .collect();
        assert_eq!(bytes, ["9.", "10.", "\u{2022}"]);
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let marker = generated
            .provenance(key(2), Utf8ByteOffset::new(0), Utf8ByteOffset::new(2))
            .unwrap();
        let receipt = binding.bind_generated_shape_text(marker).unwrap();
        assert!(receipt.covers_complete_site());
        assert!(receipt.is_standalone_logical_text());

        let wrong = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), key(2), "9. ".to_owned())
                    .unwrap(),
                package.materialize_list_marker(key(3)).unwrap(),
                package.materialize_list_marker(key(5)).unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        assert_eq!(
            package.bind_generated_text(&wrong, &limits),
            Err(PackageGeneratedTextError::ListMarkerMismatch)
        );
    }

    #[test]
    fn package_revalidates_uri_against_effective_policy() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![Inline::Link {
                node_id: NodeId::new(2),
                span,
                target: LinkTarget::Uri(
                    typaxis_core::SafeUri::new("https://example.test").unwrap(),
                ),
                children: vec![],
            }],
        });
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = vec!["mailto".to_owned()];
        assert!(matches!(
            ValidatedParsedPackage::new_entry_only(
                package,
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            ),
            Err(PackageValidationError::InvalidUri(_))
        ));
    }

    #[test]
    fn package_requires_unique_font_family_names() {
        let mut package = empty_package_with_source();
        package.resources.font_faces = vec![
            FontFaceDeclaration {
                font_face_id: FontFaceId::new(0),
                family: "Body".to_owned(),
                uri: PortablePath::new("body-a.ttf").unwrap(),
                face_index: 0,
                expected_sha256: None,
            },
            FontFaceDeclaration {
                font_face_id: FontFaceId::new(1),
                family: "Body".to_owned(),
                uri: PortablePath::new("body-b.ttf").unwrap(),
                face_index: 0,
                expected_sha256: None,
            },
        ];
        assert_eq!(
            validate(package),
            Err(PackageValidationError::DuplicateFontFamily)
        );
    }

    #[test]
    fn include_graph_checks_exact_depth_and_catalog_identity() {
        let catalog = |count: u32, suffix: &str| {
            SourceCatalog::new(
                (0..count)
                    .map(|id| {
                        SourceRecord::new(
                            SourceId::new(id),
                            PortablePath::new(format!("source-{id}-{suffix}.tsf")).unwrap(),
                            String::new(),
                        )
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap()
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_include_depth: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        let exact_catalog = catalog(2, "exact");
        let mut resolver = IncludeResolverSession::new(&exact_catalog, &limits).unwrap();
        assert_eq!(
            resolver.admit_next_include(SourceId::new(0)),
            Ok(SourceId::new(1))
        );
        let exact = resolver.finish().unwrap();
        assert_eq!(exact.max_observed_depth(), 1);

        let too_deep = catalog(3, "deep");
        let mut resolver = IncludeResolverSession::new(&too_deep, &limits).unwrap();
        assert_eq!(
            resolver.admit_next_include(SourceId::new(0)),
            Ok(SourceId::new(1))
        );
        assert_eq!(
            resolver.admit_next_include(SourceId::new(1)),
            Err(IncludeGraphError::IncludeDepthLimit)
        );
        assert!(!exact.matches(&catalog(2, "other")));
    }

    #[test]
    fn entry_only_validation_rejects_unresolved_include_syntax() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "@ include \"child.tsf\";".to_owned(),
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![]).unwrap(),
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        assert_eq!(
            ValidatedParsedPackage::new_entry_only(
                package,
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            ),
            Err(PackageValidationError::UnresolvedIncludeDirective)
        );
    }

    #[test]
    fn package_epoch_identity_matches_portable_minimal_golden() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("empty.tsf").unwrap(),
            "\n".to_owned(),
        )
        .unwrap();
        let mut package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![]).unwrap(),
        );
        let width = PositiveLength::new(Length::from_raw(39_011_981).unwrap()).unwrap();
        let height = PositiveLength::new(Length::from_raw(55_174_088).unwrap()).unwrap();
        let body_width = PositiveLength::new(Length::from_raw(31_581_127).unwrap()).unwrap();
        let body_height = PositiveLength::new(Length::from_raw(47_743_234).unwrap()).unwrap();
        package.page_masters = PageMasterSet {
            default_master_id: MasterId::new("a4").unwrap(),
            masters: vec![PageMaster {
                master_id: MasterId::new("a4").unwrap(),
                width,
                height,
                body: Rect::new(
                    Length::from_raw(3_715_427).unwrap(),
                    Length::from_raw(3_715_427).unwrap(),
                    body_width,
                    body_height,
                ),
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: vec![],
        };
        let identity = PackageEpochIdentity::from_package(&package);
        let hex = |bytes: [u8; 32]| {
            let mut value = String::new();
            push_hash_hex(&mut value, bytes);
            value.trim_matches('"').to_owned()
        };
        assert_eq!(
            hex(identity.document().bytes()),
            "8237caf0e302bfc1ec235431fa05d483aae850677633104d7ffc77e795a7e619"
        );
        assert_eq!(
            hex(identity.style().bytes()),
            "40d9810b810455a25f773560a743860c1d04e59b7c72273c161da2136b09b12d"
        );

        package.document.node_id = NodeId::new(1);
        let changed = PackageEpochIdentity::from_package(&package);
        assert_ne!(identity.document(), changed.document());
        assert_eq!(identity.style(), changed.style());
    }

    #[test]
    fn document_epoch_binds_every_resource_declaration_field() {
        let base = empty_package_with_source();
        let base_identity = PackageEpochIdentity::from_package(&base);
        let font = FontFaceDeclaration {
            font_face_id: FontFaceId::new(0),
            family: "Body".to_owned(),
            uri: PortablePath::new("body.ttf").unwrap(),
            face_index: 2,
            expected_sha256: Some([1; 32]),
        };
        let image = ImageDeclaration {
            image_id: ImageResourceId::new(0),
            uri: PortablePath::new("cover.png").unwrap(),
            expected_sha256: Some([2; 32]),
        };
        let with_resources = |font: FontFaceDeclaration, image: ImageDeclaration| {
            let mut package = base.clone();
            package.resources.font_faces.push(font);
            package.resources.images.push(image);
            PackageEpochIdentity::from_package(&package)
        };
        let identity = with_resources(font.clone(), image.clone());
        assert_ne!(base_identity.document(), identity.document());
        assert_eq!(base_identity.style(), identity.style());

        let mut variants = Vec::new();
        let mut value = font.clone();
        value.font_face_id = FontFaceId::new(1);
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.family = "Heading".to_owned();
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.uri = PortablePath::new("other.ttf").unwrap();
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.face_index = 3;
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.expected_sha256 = None;
        variants.push(with_resources(value, image.clone()));
        let mut value = image.clone();
        value.image_id = ImageResourceId::new(1);
        variants.push(with_resources(font.clone(), value));
        let mut value = image.clone();
        value.uri = PortablePath::new("other.png").unwrap();
        variants.push(with_resources(font.clone(), value));
        let mut value = image;
        value.expected_sha256 = None;
        variants.push(with_resources(font, value));

        assert!(variants
            .iter()
            .all(|variant| variant.document() != identity.document()));
        assert!(variants
            .iter()
            .all(|variant| variant.style() == identity.style()));
    }
}
