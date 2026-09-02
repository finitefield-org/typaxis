use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use typaxis_core::{
    push_jcs_string, sha256, AnchorId, JsonPointer, NodeId, SourceId, SourceSpan, Utf8ByteOffset,
    ValidatedResourceLimits,
};
use typaxis_document::{
    StagingComputedLanguageRecord, StagingDocumentMetadata, StagingLanguageNodeKind,
    StagingOutlineEntry, StagingOutlineSource, StagingOutlineSourceKind,
};
use typaxis_document_package::{
    staging_m4_wire_ast_node_count, WireAdvancedPageMasterSet, WireDocumentMetadata,
    WireOutlineSourceKind, WirePageRegion, WirePageRegionBlock, WirePageRegionInline,
    WireSourceSpan, WireStagingM4Block, WireStagingM4Document, WireStagingM4Footnote,
    WireStagingM4Inline, WireStagingM4LinkTarget, WireStagingM4TableRow, WireStagingSourceSpan,
};

use crate::{StagingSemanticSyntaxError, ValidatedStagingSemanticPackage};

pub const DOCUMENT_METADATA_ALGORITHM: &str = "typaxis.document-metadata/1";
pub const BCP47_LANGUAGE_ALGORITHM: &str = "typaxis.bcp47-language/1";
pub const COMPUTED_LANGUAGE_REGISTRY_ALGORITHM: &str = "typaxis.computed-language-registry/1";
pub const OUTLINE_REGISTRY_ALGORITHM: &str = "typaxis.outline-registry/1";
pub const BOOK_NAVIGATION_PROFILE_VIEW_ALGORITHM: &str = "typaxis.book-navigation-profile-view/1";

const GRANDFATHERED: &[&str] = &[
    "art-lojban",
    "cel-gaulish",
    "en-GB-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "no-bok",
    "no-nyn",
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookNavigationSyntaxErrorKind {
    InvalidMetadata,
    InvalidTimestamp,
    InvalidLanguage,
    InvalidOutline,
    DuplicateAnchor,
    AstNodeLimit,
    AstDepthLimit,
    TextBufferLimit,
    TextAggregateLimit,
    ReceiptMismatch,
    AllocationFailure,
    PrecomposedVectorStaging,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationSyntaxError {
    kind: BookNavigationSyntaxErrorKind,
    code: &'static str,
    pointer: JsonPointer,
    note: Option<String>,
}

impl BookNavigationSyntaxError {
    fn producer(kind: BookNavigationSyntaxErrorKind, pointer: impl Into<String>) -> Self {
        Self {
            kind,
            code: "P1102",
            pointer: pointer_from_path(&pointer.into()),
            note: None,
        }
    }

    fn limit(
        kind: BookNavigationSyntaxErrorKind,
        code: &'static str,
        pointer: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            code,
            pointer: pointer_from_path(&pointer.into()),
            note: None,
        }
    }

    fn mismatch() -> Self {
        Self {
            kind: BookNavigationSyntaxErrorKind::ReceiptMismatch,
            code: "I9190",
            pointer: JsonPointer::root(),
            note: None,
        }
    }

    fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub const fn kind(&self) -> BookNavigationSyntaxErrorKind {
        self.kind
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }

    pub const fn pointer(&self) -> &JsonPointer {
        &self.pointer
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

impl std::fmt::Display for BookNavigationSyntaxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {:?} at {}",
            self.code, self.kind, self.pointer
        )?;
        if let Some(note) = &self.note {
            write!(formatter, " ({note})")?;
        }
        Ok(())
    }
}

impl std::error::Error for BookNavigationSyntaxError {}

fn pointer_from_path(value: &str) -> JsonPointer {
    if value.is_empty() {
        JsonPointer::root()
    } else {
        JsonPointer::from_segments(value.trim_start_matches('/').split('/'))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentMetadataReceipt {
    metadata: StagingDocumentMetadata,
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl DocumentMetadataReceipt {
    pub const fn metadata(&self) -> &StagingDocumentMetadata {
        &self.metadata
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputedLanguageRegistryReceipt {
    document_language: Arc<str>,
    records: Vec<StagingComputedLanguageRecord>,
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ComputedLanguageRegistryReceipt {
    pub fn document_language(&self) -> &str {
        &self.document_language
    }
    pub fn records(&self) -> &[StagingComputedLanguageRecord] {
        &self.records
    }
    pub fn record(&self, node_id: NodeId) -> Option<&StagingComputedLanguageRecord> {
        self.records
            .binary_search_by_key(&node_id, |record| record.node_id)
            .ok()
            .map(|index| &self.records[index])
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedOutlineRegistryReceipt {
    entries: Vec<StagingOutlineEntry>,
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    language_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedOutlineRegistryReceipt {
    pub fn entries(&self) -> &[StagingOutlineEntry] {
        &self.entries
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn language_sha256(&self) -> [u8; 32] {
        self.language_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingBookNavigation {
    metadata: DocumentMetadataReceipt,
    languages: ComputedLanguageRegistryReceipt,
    outline: ValidatedOutlineRegistryReceipt,
    anchors: Vec<(AnchorId, NodeId)>,
    internal_links: Vec<(NodeId, AnchorId)>,
    limits: ValidatedResourceLimits,
}

impl ValidatedStagingBookNavigation {
    pub const fn metadata(&self) -> &DocumentMetadataReceipt {
        &self.metadata
    }
    pub const fn languages(&self) -> &ComputedLanguageRegistryReceipt {
        &self.languages
    }
    pub const fn outline(&self) -> &ValidatedOutlineRegistryReceipt {
        &self.outline
    }
    pub fn anchors(&self) -> &[(AnchorId, NodeId)] {
        &self.anchors
    }
    pub fn anchor_owner(&self, anchor_id: &AnchorId) -> Option<NodeId> {
        self.anchors
            .binary_search_by(|(anchor, _)| anchor.cmp(anchor_id))
            .ok()
            .map(|index| self.anchors[index].1)
    }
    pub fn internal_links(&self) -> &[(NodeId, AnchorId)] {
        &self.internal_links
    }
    pub fn internal_link_target(&self, owner: NodeId) -> Option<&AnchorId> {
        self.internal_links
            .binary_search_by_key(&owner, |(candidate, _)| *candidate)
            .ok()
            .map(|index| &self.internal_links[index].1)
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), BookNavigationSyntaxError> {
        let observed = validate_staging_book_navigation_inner(package, limits)?;
        if self != &observed {
            return Err(BookNavigationSyntaxError::mismatch());
        }
        Ok(())
    }
}

/// Dependency-inversion receipt consumed by the profile owner and downstream
/// staging crates without creating a syntax -> profile dependency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingBookNavigationProfileView {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
    language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingBookNavigationProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, BookNavigationSyntaxError> {
        navigation.verify(package, limits)?;
        let mut value = Self {
            package_sha256: package.canonical_jcs_sha256(),
            semantic_sha256: package.semantic_fingerprint(),
            metadata_sha256: navigation.metadata.fingerprint,
            language_sha256: navigation.languages.fingerprint,
            outline_sha256: navigation.outline.fingerprint,
            limits_sha256: limits_fingerprint(limits),
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        value.canonical_jcs = encode_profile_view(&value);
        value.fingerprint = sha256(value.canonical_jcs.as_bytes());
        Ok(value)
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn metadata_sha256(&self) -> [u8; 32] {
        self.metadata_sha256
    }
    pub const fn language_sha256(&self) -> [u8; 32] {
        self.language_sha256
    }
    pub const fn outline_sha256(&self) -> [u8; 32] {
        self.outline_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), BookNavigationSyntaxError> {
        navigation.verify(package, limits)?;
        let observed = Self::new(package, navigation, limits)?;
        if &observed != self {
            return Err(BookNavigationSyntaxError::mismatch());
        }
        Ok(())
    }
}

/// Dependency-inversion projection issued only after machine-profile
/// preflight has sealed its receipt. Downstream crates bind to the profile
/// receipt fingerprint without depending on the profile crate itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingBookNavigationProfileAuthorization {
    view: StagingBookNavigationProfileView,
    profile_receipt_fingerprint: [u8; 32],
}

impl StagingBookNavigationProfileAuthorization {
    #[doc(hidden)]
    pub fn bind_profile_receipt(
        view: StagingBookNavigationProfileView,
        profile_receipt_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, BookNavigationSyntaxError> {
        let expected = StagingBookNavigationProfileView::new(package, navigation, limits)?;
        if view != expected || profile_receipt_fingerprint == [0; 32] {
            return Err(BookNavigationSyntaxError::mismatch());
        }
        Ok(Self {
            view,
            profile_receipt_fingerprint,
        })
    }

    pub const fn view(&self) -> &StagingBookNavigationProfileView {
        &self.view
    }

    pub const fn profile_receipt_fingerprint(&self) -> [u8; 32] {
        self.profile_receipt_fingerprint
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.view.fingerprint()
    }

    pub const fn metadata_sha256(&self) -> [u8; 32] {
        self.view.metadata_sha256()
    }

    pub const fn language_sha256(&self) -> [u8; 32] {
        self.view.language_sha256()
    }

    pub const fn outline_sha256(&self) -> [u8; 32] {
        self.view.outline_sha256()
    }

    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.view.limits_sha256()
    }

    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), BookNavigationSyntaxError> {
        self.view.authorizes(package, navigation, limits)
    }
}

#[derive(Clone, Debug)]
struct LanguageSite {
    node_id: u32,
    kind: StagingLanguageNodeKind,
    parent: Option<u32>,
    span: Option<WireStagingSourceSpan>,
    raw: Option<String>,
    pointer: String,
}

#[derive(Clone, Debug)]
struct OutlineOwner {
    kind: StagingOutlineSourceKind,
    node_id: u32,
    span: WireStagingSourceSpan,
    anchor: Option<String>,
    heading_level: Option<u8>,
    semantic_kind: Option<String>,
}

pub fn validate_staging_book_navigation(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
) -> Result<ValidatedStagingBookNavigation, BookNavigationSyntaxError> {
    validate_staging_book_navigation_inner(package, limits)
}

fn validate_staging_book_navigation_inner(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
) -> Result<ValidatedStagingBookNavigation, BookNavigationSyntaxError> {
    if package.limits() != limits {
        return Err(BookNavigationSyntaxError::mismatch());
    }
    let wire = package.checked_wire().map_err(map_semantic_error)?;
    let limits_sha256 = limits_fingerprint(limits);
    let package_sha256 = package.canonical_jcs_sha256();

    let metadata = validate_metadata(wire.metadata(), package_sha256, limits_sha256, limits)?;

    let mut sites = Vec::new();
    let mut owners = BTreeMap::new();
    let mut anchors: BTreeMap<String, (u32, String)> = BTreeMap::new();
    collect_document(
        wire.document(),
        wire.advanced_page_masters(),
        &mut sites,
        &mut owners,
        &mut anchors,
    )?;
    let language_charges = sites
        .iter()
        .map(|site| (site.raw.clone(), site.pointer.clone()))
        .collect::<Vec<_>>();
    let languages = validate_languages(sites, package_sha256, limits_sha256, wire, limits)?;
    let outline = validate_outline(
        wire,
        &owners,
        &anchors,
        &languages,
        package.semantic_fingerprint(),
        package_sha256,
        limits_sha256,
        limits,
    )?;
    let anchors = anchors
        .iter()
        .map(|(anchor, (owner, _))| {
            AnchorId::new(anchor.clone())
                .map(|anchor| (anchor, NodeId::new(*owner)))
                .map_err(|_| BookNavigationSyntaxError::mismatch())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let internal_links = collect_internal_links(wire.document(), &anchors)?;
    validate_aggregate_text(wire, &languages, &language_charges, limits)?;
    validate_navigation_node_limits(wire, package, limits)?;

    Ok(ValidatedStagingBookNavigation {
        metadata,
        languages,
        outline,
        anchors,
        internal_links,
        limits: limits.clone(),
    })
}

fn collect_internal_links(
    document: &WireStagingM4Document,
    anchors: &[(AnchorId, NodeId)],
) -> Result<Vec<(NodeId, AnchorId)>, BookNavigationSyntaxError> {
    fn inlines(
        values: &[WireStagingM4Inline],
        pointer: &str,
        output: &mut Vec<(NodeId, AnchorId, String)>,
    ) -> Result<(), BookNavigationSyntaxError> {
        for (index, value) in values.iter().enumerate() {
            let base = format!("{pointer}/{index}");
            if let WireStagingM4Inline::Link {
                node_id,
                target: WireStagingM4LinkTarget::Internal { anchor_id },
                ..
            } = value
            {
                output.push((
                    NodeId::new(*node_id),
                    AnchorId::new(anchor_id.clone()).map_err(|_| {
                        BookNavigationSyntaxError::producer(
                            BookNavigationSyntaxErrorKind::InvalidOutline,
                            format!("{base}/target/anchor_id"),
                        )
                    })?,
                    format!("{base}/target/anchor_id"),
                ));
            }
            match value {
                WireStagingM4Inline::Emphasis { children, .. }
                | WireStagingM4Inline::Strong { children, .. }
                | WireStagingM4Inline::Link { children, .. } => {
                    inlines(children, &format!("{base}/children"), output)?;
                }
                WireStagingM4Inline::InlineVector { .. }
                | WireStagingM4Inline::MathVector { .. } => {
                    return Err(BookNavigationSyntaxError::producer(
                        BookNavigationSyntaxErrorKind::PrecomposedVectorStaging,
                        base,
                    ));
                }
                WireStagingM4Inline::Text { .. }
                | WireStagingM4Inline::InlineMath { .. }
                | WireStagingM4Inline::Anchor { .. }
                | WireStagingM4Inline::Reference { .. }
                | WireStagingM4Inline::FootnoteReference { .. }
                | WireStagingM4Inline::SoftBreak { .. }
                | WireStagingM4Inline::HardBreak { .. } => {}
            }
        }
        Ok(())
    }

    fn blocks(
        values: &[WireStagingM4Block],
        pointer: &str,
        output: &mut Vec<(NodeId, AnchorId, String)>,
    ) -> Result<(), BookNavigationSyntaxError> {
        for (index, value) in values.iter().enumerate() {
            let base = format!("{pointer}/{index}");
            match value {
                WireStagingM4Block::Paragraph { children, .. }
                | WireStagingM4Block::Heading { children, .. } => {
                    inlines(children, &format!("{base}/children"), output)?;
                }
                WireStagingM4Block::List { items, .. } => {
                    for (item_index, item) in items.iter().enumerate() {
                        blocks(
                            &item.blocks,
                            &format!("{base}/items/{item_index}/blocks"),
                            output,
                        )?;
                    }
                }
                WireStagingM4Block::Table { head, body, .. } => {
                    for (collection, rows) in [("head", head), ("body", body)] {
                        for (row_index, row) in rows.iter().enumerate() {
                            for (cell_index, cell) in row.cells.iter().enumerate() {
                                blocks(
                                    &cell.blocks,
                                    &format!(
                                        "{base}/{collection}/{row_index}/cells/{cell_index}/blocks"
                                    ),
                                    output,
                                )?;
                            }
                        }
                    }
                }
                WireStagingM4Block::Figure { caption, .. } => {
                    blocks(caption, &format!("{base}/caption"), output)?;
                }
                WireStagingM4Block::SemanticContainer {
                    blocks: children, ..
                } => {
                    blocks(children, &format!("{base}/blocks"), output)?;
                }
                WireStagingM4Block::VectorFigure { .. }
                | WireStagingM4Block::MathVectorBlock { .. } => {
                    return Err(BookNavigationSyntaxError::producer(
                        BookNavigationSyntaxErrorKind::PrecomposedVectorStaging,
                        base,
                    ));
                }
                WireStagingM4Block::PageBreak { .. } | WireStagingM4Block::DisplayMath { .. } => {}
            }
        }
        Ok(())
    }

    let mut raw = Vec::new();
    blocks(&document.blocks, "/document/blocks", &mut raw)?;
    for (index, footnote) in document.footnotes.iter().enumerate() {
        blocks(
            &footnote.blocks,
            &format!("/document/footnotes/{index}/blocks"),
            &mut raw,
        )?;
    }
    raw.sort_by_key(|(owner, _, _)| *owner);
    if raw.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(BookNavigationSyntaxError::mismatch());
    }
    let mut output = Vec::new();
    output.try_reserve_exact(raw.len()).map_err(|_| {
        BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::AllocationFailure,
            "P1120",
            "/document",
        )
    })?;
    for (owner, target, pointer) in raw {
        if anchors
            .binary_search_by(|(anchor, _)| anchor.cmp(&target))
            .is_err()
        {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                pointer,
            ));
        }
        output.push((owner, target));
    }
    Ok(output)
}

fn map_semantic_error(_: StagingSemanticSyntaxError) -> BookNavigationSyntaxError {
    BookNavigationSyntaxError::mismatch()
}

fn validate_metadata(
    wire: &WireDocumentMetadata,
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    limits: &ValidatedResourceLimits,
) -> Result<DocumentMetadataReceipt, BookNavigationSyntaxError> {
    if let Some(author) = &wire.author {
        validate_metadata_string(author, "/metadata/author", limits)?;
    }
    if let Some(created) = &wire.created {
        validate_timestamp(created, "/metadata/created", limits)?;
    }
    if let Some(identifier) = &wire.identifier {
        validate_metadata_string(identifier, "/metadata/identifier", limits)?;
    }
    let mut previous: Option<&[u8]> = None;
    for (index, keyword) in wire.keywords.iter().enumerate() {
        let pointer = format!("/metadata/keywords/{index}");
        validate_metadata_string(keyword, &pointer, limits)?;
        if previous.is_some_and(|prior| prior >= keyword.as_bytes()) {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidMetadata,
                pointer,
            ));
        }
        previous = Some(keyword.as_bytes());
    }
    if let Some(modified) = &wire.modified {
        validate_timestamp(modified, "/metadata/modified", limits)?;
    }
    if let (Some(created), Some(modified)) = (&wire.created, &wire.modified) {
        if modified < created {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidTimestamp,
                "/metadata/modified",
            ));
        }
    }
    if let Some(subject) = &wire.subject {
        validate_metadata_string(subject, "/metadata/subject", limits)?;
    }
    if let Some(title) = &wire.title {
        validate_metadata_string(title, "/metadata/title", limits)?;
    }
    let metadata = StagingDocumentMetadata {
        author: wire.author.clone(),
        created: wire.created.clone(),
        identifier: wire.identifier.clone(),
        keywords: wire.keywords.clone(),
        modified: wire.modified.clone(),
        subject: wire.subject.clone(),
        title: wire.title.clone(),
    };
    let canonical_jcs = encode_metadata(&metadata, package_sha256, limits_sha256);
    Ok(DocumentMetadataReceipt {
        metadata,
        package_sha256,
        limits_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn validate_metadata_string(
    value: &str,
    pointer: &str,
    limits: &ValidatedResourceLimits,
) -> Result<(), BookNavigationSyntaxError> {
    if u64::try_from(value.len()).map_or(true, |length| {
        length > u64::from(limits.get().max_text_buffer_bytes)
    }) {
        return Err(BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::TextBufferLimit,
            "T2100",
            pointer,
        ));
    }
    if value.is_empty()
        || value.chars().all(is_unicode_16_white_space)
        || value.chars().any(|scalar| {
            matches!(scalar, '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}' | '\u{fffe}' | '\u{ffff}')
        })
    {
        return Err(BookNavigationSyntaxError::producer(
            BookNavigationSyntaxErrorKind::InvalidMetadata,
            pointer,
        ));
    }
    Ok(())
}

fn validate_timestamp(
    value: &str,
    pointer: &str,
    limits: &ValidatedResourceLimits,
) -> Result<(), BookNavigationSyntaxError> {
    if u64::try_from(value.len()).map_or(true, |length| {
        length > u64::from(limits.get().max_text_buffer_bytes)
    }) {
        return Err(BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::TextBufferLimit,
            "T2100",
            pointer,
        ));
    }
    let bytes = value.as_bytes();
    let invalid_shape = bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 4 | 7 | 10 | 13 | 16 | 19) && !byte.is_ascii_digit()
        });
    if invalid_shape {
        return Err(BookNavigationSyntaxError::producer(
            BookNavigationSyntaxErrorKind::InvalidTimestamp,
            pointer,
        ));
    }
    let number = |start: usize, end: usize| -> u32 {
        bytes[start..end]
            .iter()
            .fold(0, |value, byte| value * 10 + u32::from(byte - b'0'))
    };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day || hour > 23 || minute > 59 || second > 59 {
        return Err(BookNavigationSyntaxError::producer(
            BookNavigationSyntaxErrorKind::InvalidTimestamp,
            pointer,
        ));
    }
    Ok(())
}

fn collect_document(
    document: &WireStagingM4Document,
    page_masters: &WireAdvancedPageMasterSet,
    sites: &mut Vec<LanguageSite>,
    owners: &mut BTreeMap<u32, OutlineOwner>,
    anchors: &mut BTreeMap<String, (u32, String)>,
) -> Result<(), BookNavigationSyntaxError> {
    sites.push(LanguageSite {
        node_id: document.node_id,
        kind: StagingLanguageNodeKind::Document,
        parent: None,
        span: None,
        raw: Some(document.language.clone()),
        pointer: "/document/language".to_owned(),
    });
    collect_blocks(
        &document.blocks,
        document.node_id,
        "/document/blocks",
        true,
        sites,
        owners,
        anchors,
    )?;
    for (index, footnote) in document.footnotes.iter().enumerate() {
        collect_footnote(
            footnote,
            document.node_id,
            &format!("/document/footnotes/{index}"),
            sites,
            owners,
            anchors,
        )?;
    }
    collect_page_regions(page_masters, document.node_id, sites)?;
    sites.sort_by_key(|site| site.node_id);
    if sites
        .windows(2)
        .any(|pair| pair[0].node_id >= pair[1].node_id)
    {
        return Err(BookNavigationSyntaxError::mismatch());
    }
    Ok(())
}

fn collect_page_regions(
    page_masters: &WireAdvancedPageMasterSet,
    document_node_id: u32,
    sites: &mut Vec<LanguageSite>,
) -> Result<(), BookNavigationSyntaxError> {
    for (master_index, master) in page_masters.masters.iter().enumerate() {
        for (name, region) in [
            ("header_content", master.header_content.as_ref()),
            ("footer_content", master.footer_content.as_ref()),
        ] {
            if let Some(region) = region {
                collect_page_region(
                    region,
                    document_node_id,
                    &format!("/page_masters/masters/{master_index}/{name}"),
                    sites,
                )?;
            }
        }
    }
    Ok(())
}

fn collect_page_region(
    region: &WirePageRegion,
    document_node_id: u32,
    pointer: &str,
    sites: &mut Vec<LanguageSite>,
) -> Result<(), BookNavigationSyntaxError> {
    for (block_index, block) in region.blocks.iter().enumerate() {
        let block_pointer = format!("{pointer}/blocks/{block_index}");
        let (node_id, span, kind, children) = match block {
            WirePageRegionBlock::Paragraph {
                node_id,
                span,
                children,
                ..
            } => (
                *node_id,
                *span,
                StagingLanguageNodeKind::Paragraph,
                children,
            ),
            WirePageRegionBlock::Heading {
                node_id,
                span,
                children,
                ..
            } => (*node_id, *span, StagingLanguageNodeKind::Heading, children),
        };
        sites.push(LanguageSite {
            node_id,
            kind,
            parent: Some(document_node_id),
            span: Some(staging_span(span)),
            raw: None,
            pointer: format!("{block_pointer}/language"),
        });
        for (inline_index, inline) in children.iter().enumerate() {
            if let WirePageRegionInline::Text {
                node_id: child,
                span,
                ..
            } = inline
            {
                sites.push(LanguageSite {
                    node_id: *child,
                    kind: StagingLanguageNodeKind::Text,
                    parent: Some(node_id),
                    span: Some(staging_span(*span)),
                    raw: None,
                    pointer: format!("{block_pointer}/children/{inline_index}/language"),
                });
            }
        }
    }
    Ok(())
}

fn staging_span(value: WireSourceSpan) -> WireStagingSourceSpan {
    WireStagingSourceSpan {
        source_id: value.source_id,
        start_byte: value.start_byte,
        end_byte: value.end_byte,
    }
}

fn collect_footnote(
    footnote: &WireStagingM4Footnote,
    parent: u32,
    pointer: &str,
    sites: &mut Vec<LanguageSite>,
    owners: &mut BTreeMap<u32, OutlineOwner>,
    anchors: &mut BTreeMap<String, (u32, String)>,
) -> Result<(), BookNavigationSyntaxError> {
    sites.push(LanguageSite {
        node_id: footnote.node_id,
        kind: StagingLanguageNodeKind::FootnoteDefinition,
        parent: Some(parent),
        span: Some(footnote.span),
        raw: footnote.language.clone(),
        pointer: format!("{pointer}/language"),
    });
    collect_blocks(
        &footnote.blocks,
        footnote.node_id,
        &format!("{pointer}/blocks"),
        true,
        sites,
        owners,
        anchors,
    )
}

#[allow(clippy::too_many_arguments)]
fn collect_blocks(
    blocks: &[WireStagingM4Block],
    parent: u32,
    pointer: &str,
    outline_eligible: bool,
    sites: &mut Vec<LanguageSite>,
    owners: &mut BTreeMap<u32, OutlineOwner>,
    anchors: &mut BTreeMap<String, (u32, String)>,
) -> Result<(), BookNavigationSyntaxError> {
    for (index, block) in blocks.iter().enumerate() {
        let base = format!("{pointer}/{index}");
        let node_id = block.node_id();
        let span = raw_block_span(block);
        let (kind, raw) = match block {
            WireStagingM4Block::Paragraph { language, .. } => {
                (Some(StagingLanguageNodeKind::Paragraph), language)
            }
            WireStagingM4Block::Heading {
                language,
                level,
                anchor_id,
                ..
            } => {
                if let Some(anchor) = anchor_id {
                    insert_anchor(anchors, anchor, node_id, format!("{base}/anchor_id"))?;
                }
                if outline_eligible {
                    owners.insert(
                        node_id,
                        OutlineOwner {
                            kind: StagingOutlineSourceKind::Heading,
                            node_id,
                            span,
                            anchor: anchor_id.clone(),
                            heading_level: Some(*level),
                            semantic_kind: None,
                        },
                    );
                }
                (Some(StagingLanguageNodeKind::Heading), language)
            }
            WireStagingM4Block::List { language, .. } => {
                (Some(StagingLanguageNodeKind::List), language)
            }
            WireStagingM4Block::Table { language, .. } => {
                (Some(StagingLanguageNodeKind::Table), language)
            }
            WireStagingM4Block::Figure { language, .. } => {
                (Some(StagingLanguageNodeKind::Figure), language)
            }
            WireStagingM4Block::DisplayMath { language, .. } => {
                (Some(StagingLanguageNodeKind::DisplayMath), language)
            }
            WireStagingM4Block::VectorFigure { .. }
            | WireStagingM4Block::MathVectorBlock { .. } => {
                return Err(BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::PrecomposedVectorStaging,
                    base,
                ));
            }
            WireStagingM4Block::SemanticContainer {
                language,
                anchor_id,
                semantic_kind,
                ..
            } => {
                if let Some(anchor) = anchor_id {
                    insert_anchor(anchors, anchor, node_id, format!("{base}/anchor_id"))?;
                }
                if outline_eligible {
                    owners.insert(
                        node_id,
                        OutlineOwner {
                            kind: StagingOutlineSourceKind::SemanticContainer,
                            node_id,
                            span,
                            anchor: anchor_id.clone(),
                            heading_level: None,
                            semantic_kind: Some(semantic_kind.as_str().to_owned()),
                        },
                    );
                }
                (Some(StagingLanguageNodeKind::SemanticContainer), language)
            }
            WireStagingM4Block::PageBreak { .. } => (None, &None),
        };
        if let Some(kind) = kind {
            sites.push(LanguageSite {
                node_id,
                kind,
                parent: Some(parent),
                span: Some(span),
                raw: raw.clone(),
                pointer: format!("{base}/language"),
            });
        }
        match block {
            WireStagingM4Block::Paragraph { children, .. }
            | WireStagingM4Block::Heading { children, .. } => collect_inlines(
                children,
                node_id,
                &format!("{base}/children"),
                sites,
                anchors,
            )?,
            WireStagingM4Block::List { items, .. } => {
                for (item_index, item) in items.iter().enumerate() {
                    let item_pointer = format!("{base}/items/{item_index}");
                    sites.push(LanguageSite {
                        node_id: item.node_id,
                        kind: StagingLanguageNodeKind::ListItem,
                        parent: Some(node_id),
                        span: Some(item.span),
                        raw: item.language.clone(),
                        pointer: format!("{item_pointer}/language"),
                    });
                    collect_blocks(
                        &item.blocks,
                        item.node_id,
                        &format!("{item_pointer}/blocks"),
                        outline_eligible,
                        sites,
                        owners,
                        anchors,
                    )?;
                }
            }
            WireStagingM4Block::Table { head, body, .. } => {
                collect_rows(
                    head,
                    node_id,
                    &format!("{base}/head"),
                    outline_eligible,
                    sites,
                    owners,
                    anchors,
                )?;
                collect_rows(
                    body,
                    node_id,
                    &format!("{base}/body"),
                    outline_eligible,
                    sites,
                    owners,
                    anchors,
                )?;
            }
            WireStagingM4Block::Figure { caption, .. } => collect_blocks(
                caption,
                node_id,
                &format!("{base}/caption"),
                outline_eligible,
                sites,
                owners,
                anchors,
            )?,
            WireStagingM4Block::SemanticContainer { blocks, .. } => collect_blocks(
                blocks,
                node_id,
                &format!("{base}/blocks"),
                outline_eligible,
                sites,
                owners,
                anchors,
            )?,
            WireStagingM4Block::VectorFigure { .. }
            | WireStagingM4Block::MathVectorBlock { .. } => {
                return Err(BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::PrecomposedVectorStaging,
                    base,
                ));
            }
            WireStagingM4Block::PageBreak { .. } | WireStagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_rows(
    rows: &[WireStagingM4TableRow],
    parent: u32,
    pointer: &str,
    outline_eligible: bool,
    sites: &mut Vec<LanguageSite>,
    owners: &mut BTreeMap<u32, OutlineOwner>,
    anchors: &mut BTreeMap<String, (u32, String)>,
) -> Result<(), BookNavigationSyntaxError> {
    for (row_index, row) in rows.iter().enumerate() {
        let row_pointer = format!("{pointer}/{row_index}");
        sites.push(LanguageSite {
            node_id: row.node_id,
            kind: StagingLanguageNodeKind::TableRow,
            parent: Some(parent),
            span: Some(row.span),
            raw: row.language.clone(),
            pointer: format!("{row_pointer}/language"),
        });
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let cell_pointer = format!("{row_pointer}/cells/{cell_index}");
            sites.push(LanguageSite {
                node_id: cell.node_id,
                kind: StagingLanguageNodeKind::TableCell,
                parent: Some(row.node_id),
                span: Some(cell.span),
                raw: cell.language.clone(),
                pointer: format!("{cell_pointer}/language"),
            });
            collect_blocks(
                &cell.blocks,
                cell.node_id,
                &format!("{cell_pointer}/blocks"),
                outline_eligible,
                sites,
                owners,
                anchors,
            )?;
        }
    }
    Ok(())
}

fn collect_inlines(
    inlines: &[WireStagingM4Inline],
    parent: u32,
    pointer: &str,
    sites: &mut Vec<LanguageSite>,
    anchors: &mut BTreeMap<String, (u32, String)>,
) -> Result<(), BookNavigationSyntaxError> {
    for (index, inline) in inlines.iter().enumerate() {
        let base = format!("{pointer}/{index}");
        let node_id = inline.node_id();
        let kind = match inline {
            WireStagingM4Inline::Text { .. } => Some(StagingLanguageNodeKind::Text),
            WireStagingM4Inline::InlineMath { .. } => Some(StagingLanguageNodeKind::InlineMath),
            WireStagingM4Inline::InlineVector { .. } | WireStagingM4Inline::MathVector { .. } => {
                return Err(BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::PrecomposedVectorStaging,
                    base,
                ));
            }
            WireStagingM4Inline::Emphasis { .. } => Some(StagingLanguageNodeKind::Emphasis),
            WireStagingM4Inline::Strong { .. } => Some(StagingLanguageNodeKind::Strong),
            WireStagingM4Inline::Link { .. } => Some(StagingLanguageNodeKind::Link),
            WireStagingM4Inline::Reference { .. } => Some(StagingLanguageNodeKind::Reference),
            WireStagingM4Inline::FootnoteReference { .. } => {
                Some(StagingLanguageNodeKind::FootnoteReference)
            }
            WireStagingM4Inline::Anchor { anchor_id, .. } => {
                insert_anchor(anchors, anchor_id, node_id, format!("{base}/anchor_id"))?;
                None
            }
            WireStagingM4Inline::SoftBreak { .. } | WireStagingM4Inline::HardBreak { .. } => None,
        };
        if let Some(kind) = kind {
            sites.push(LanguageSite {
                node_id,
                kind,
                parent: Some(parent),
                span: Some(inline.span()),
                raw: inline.language().map(str::to_owned),
                pointer: format!("{base}/language"),
            });
        }
        match inline {
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } => {
                collect_inlines(
                    children,
                    node_id,
                    &format!("{base}/children"),
                    sites,
                    anchors,
                )?;
            }
            WireStagingM4Inline::InlineVector { .. } | WireStagingM4Inline::MathVector { .. } => {
                return Err(BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::PrecomposedVectorStaging,
                    base,
                ));
            }
            WireStagingM4Inline::Text { .. }
            | WireStagingM4Inline::InlineMath { .. }
            | WireStagingM4Inline::Anchor { .. }
            | WireStagingM4Inline::Reference { .. }
            | WireStagingM4Inline::FootnoteReference { .. }
            | WireStagingM4Inline::SoftBreak { .. }
            | WireStagingM4Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
}

fn insert_anchor(
    anchors: &mut BTreeMap<String, (u32, String)>,
    value: &str,
    owner: u32,
    pointer: String,
) -> Result<(), BookNavigationSyntaxError> {
    AnchorId::new(value.to_owned()).map_err(|_| {
        BookNavigationSyntaxError::producer(
            BookNavigationSyntaxErrorKind::InvalidOutline,
            pointer.clone(),
        )
    })?;
    if let Some((first_owner, first_pointer)) =
        anchors.insert(value.to_owned(), (owner, pointer.clone()))
    {
        return Err(BookNavigationSyntaxError::producer(
            BookNavigationSyntaxErrorKind::DuplicateAnchor,
            pointer,
        )
        .with_note(format!("first owner node {first_owner} at {first_pointer}")));
    }
    Ok(())
}

fn validate_languages(
    sites: Vec<LanguageSite>,
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    _wire: &typaxis_document_package::WireStagingM4DocumentPackage,
    limits: &ValidatedResourceLimits,
) -> Result<ComputedLanguageRegistryReceipt, BookNavigationSyntaxError> {
    let mut language_pool: BTreeSet<Arc<str>> = BTreeSet::new();
    let mut effective_by_node: BTreeMap<u32, Arc<str>> = BTreeMap::new();
    let mut records = Vec::new();
    records.try_reserve_exact(sites.len()).map_err(|_| {
        BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::AllocationFailure,
            "P1120",
            "/document",
        )
    })?;
    for site in sites {
        let explicit = match &site.raw {
            Some(raw) => Some(intern_language(
                &mut language_pool,
                canonicalize_language(raw, &site.pointer, limits)?,
            )),
            None => None,
        };
        let effective = match (&explicit, site.parent) {
            (Some(value), _) => value.clone(),
            (None, Some(parent)) => effective_by_node
                .get(&parent)
                .cloned()
                .ok_or_else(BookNavigationSyntaxError::mismatch)?,
            (None, None) => return Err(BookNavigationSyntaxError::mismatch()),
        };
        effective_by_node.insert(site.node_id, effective.clone());
        records.push(StagingComputedLanguageRecord {
            node_id: NodeId::new(site.node_id),
            node_kind: site.kind,
            logical_parent_node_id: site.parent.map(NodeId::new),
            source_span: site.span.map(lower_span).transpose()?,
            explicit_language: explicit,
            effective_language: effective,
        });
    }
    let document_language = records
        .first()
        .filter(|record| record.node_kind == StagingLanguageNodeKind::Document)
        .map(|record| record.effective_language.clone())
        .ok_or_else(BookNavigationSyntaxError::mismatch)?;
    let canonical_jcs =
        encode_languages(&document_language, &records, package_sha256, limits_sha256);
    Ok(ComputedLanguageRegistryReceipt {
        document_language,
        records,
        package_sha256,
        limits_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn intern_language(pool: &mut BTreeSet<Arc<str>>, value: String) -> Arc<str> {
    if let Some(existing) = pool.get(value.as_str()) {
        return existing.clone();
    }
    let value: Arc<str> = value.into();
    pool.insert(value.clone());
    value
}

pub fn canonicalize_bcp47_language(value: &str) -> Result<String, BookNavigationSyntaxError> {
    canonicalize_language_with_limit(value, "/language", u64::MAX)
}

fn canonicalize_language(
    value: &str,
    pointer: &str,
    limits: &ValidatedResourceLimits,
) -> Result<String, BookNavigationSyntaxError> {
    canonicalize_language_with_limit(
        value,
        pointer,
        u64::from(limits.get().max_text_buffer_bytes),
    )
}

fn canonicalize_language_with_limit(
    value: &str,
    pointer: &str,
    max_text_buffer_bytes: u64,
) -> Result<String, BookNavigationSyntaxError> {
    let invalid = || {
        BookNavigationSyntaxError::producer(BookNavigationSyntaxErrorKind::InvalidLanguage, pointer)
    };
    let exceeds_text_limit =
        u64::try_from(value.len()).map_or(true, |length| length > max_text_buffer_bytes);
    if value.is_empty()
        || value.len() > 255
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-'))
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return Err(invalid());
    }
    if exceeds_text_limit {
        return Err(BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::TextBufferLimit,
            "T2100",
            pointer,
        ));
    }
    if let Some(canonical) = GRANDFATHERED
        .iter()
        .find(|canonical| canonical.eq_ignore_ascii_case(value))
    {
        return Ok((*canonical).to_owned());
    }
    let parts: Vec<&str> = value.split('-').collect();
    if parts[0].eq_ignore_ascii_case("x") {
        if parts.len() < 2
            || parts[1..]
                .iter()
                .any(|part| part.is_empty() || part.len() > 8 || !is_alnum(part))
        {
            return Err(invalid());
        }
        return Ok(parts
            .iter()
            .map(|part| part.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join("-"));
    }
    let primary = parts[0];
    if !is_alpha(primary) || !(2..=8).contains(&primary.len()) {
        return Err(invalid());
    }
    let mut index = 1usize;
    let mut core = vec![primary.to_ascii_lowercase()];
    if primary.len() <= 3 {
        let mut extlang_count = 0;
        while index < parts.len()
            && extlang_count < 3
            && parts[index].len() == 3
            && is_alpha(parts[index])
        {
            core.push(parts[index].to_ascii_lowercase());
            index += 1;
            extlang_count += 1;
        }
    }
    if index < parts.len() && parts[index].len() == 4 && is_alpha(parts[index]) {
        let lower = parts[index].to_ascii_lowercase();
        let mut chars = lower.chars();
        let first = chars.next().ok_or_else(invalid)?.to_ascii_uppercase();
        core.push(format!("{first}{}", chars.as_str()));
        index += 1;
    }
    if index < parts.len()
        && ((parts[index].len() == 2 && is_alpha(parts[index]))
            || (parts[index].len() == 3 && is_digit(parts[index])))
    {
        core.push(if is_alpha(parts[index]) {
            parts[index].to_ascii_uppercase()
        } else {
            parts[index].to_owned()
        });
        index += 1;
    }
    let mut variants = BTreeSet::new();
    while index < parts.len() && is_variant(parts[index]) {
        let variant = parts[index].to_ascii_lowercase();
        if !variants.insert(variant.clone()) {
            return Err(invalid());
        }
        core.push(variant);
        index += 1;
    }
    let mut extensions: Vec<(String, Vec<String>)> = Vec::new();
    let mut singletons = BTreeSet::new();
    while index < parts.len()
        && is_singleton(parts[index])
        && !parts[index].eq_ignore_ascii_case("x")
    {
        let singleton = parts[index].to_ascii_lowercase();
        if !singletons.insert(singleton.clone()) {
            return Err(invalid());
        }
        index += 1;
        let start = index;
        let mut subtags = Vec::new();
        while index < parts.len() && (2..=8).contains(&parts[index].len()) && is_alnum(parts[index])
        {
            subtags.push(parts[index].to_ascii_lowercase());
            index += 1;
        }
        if index == start {
            return Err(invalid());
        }
        extensions.push((singleton, subtags));
    }
    let private_use = if index < parts.len() && parts[index].eq_ignore_ascii_case("x") {
        index += 1;
        if index == parts.len()
            || parts[index..]
                .iter()
                .any(|part| part.is_empty() || part.len() > 8 || !is_alnum(part))
        {
            return Err(invalid());
        }
        Some(
            parts[index..]
                .iter()
                .map(|part| part.to_ascii_lowercase())
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    if private_use.is_none() && index != parts.len() {
        return Err(invalid());
    }
    extensions.sort_by(|left, right| left.0.cmp(&right.0));
    for (singleton, subtags) in extensions {
        core.push(singleton);
        core.extend(subtags);
    }
    if let Some(private) = private_use {
        core.push("x".to_owned());
        core.extend(private);
    }
    let canonical = core.join("-");
    if canonical.len() > 255 {
        return Err(invalid());
    }
    Ok(canonical)
}

fn is_alpha(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_alphabetic())
}
fn is_digit(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_digit())
}
fn is_alnum(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}
fn is_variant(value: &str) -> bool {
    ((5..=8).contains(&value.len()) && is_alnum(value))
        || (value.len() == 4 && value.as_bytes()[0].is_ascii_digit() && is_alnum(value))
}
fn is_singleton(value: &str) -> bool {
    value.len() == 1 && value.as_bytes()[0].is_ascii_alphanumeric()
}

#[allow(clippy::too_many_arguments)]
fn validate_outline(
    wire: &typaxis_document_package::WireStagingM4DocumentPackage,
    owners: &BTreeMap<u32, OutlineOwner>,
    anchors: &BTreeMap<String, (u32, String)>,
    languages: &ComputedLanguageRegistryReceipt,
    semantic_sha256: [u8; 32],
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    limits: &ValidatedResourceLimits,
) -> Result<ValidatedOutlineRegistryReceipt, BookNavigationSyntaxError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(wire.outline().entries.len())
        .map_err(|_| {
            BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::AllocationFailure,
                "P1120",
                "/outline/entries",
            )
        })?;
    let mut stack: Vec<(u8, u32)> = Vec::new();
    let mut sources = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    let mut previous_source = None;
    for (index, entry) in wire.outline().entries.iter().enumerate() {
        let base = format!("/outline/entries/{index}");
        let destination = AnchorId::new(entry.destination.clone()).map_err(|_| {
            BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/destination"),
            )
        })?;
        if !destinations.insert(entry.destination.as_str()) {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/destination"),
            ));
        }
        let owner = owners.get(&entry.source_node_id);
        if let Some(owner) = owner {
            let anchor = owner.anchor.as_deref().ok_or_else(|| {
                BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::InvalidOutline,
                    format!("{base}/destination"),
                )
            })?;
            if anchor != entry.destination
                || anchors.get(&entry.destination).map(|value| value.0) != Some(owner.node_id)
            {
                return Err(BookNavigationSyntaxError::producer(
                    BookNavigationSyntaxErrorKind::InvalidOutline,
                    format!("{base}/destination"),
                ));
            }
        }
        validate_metadata_string(&entry.label, &format!("{base}/label"), limits)?;
        if !(1..=6).contains(&entry.level) {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/level"),
            ));
        }
        let depth = 2u32 + u32::from(entry.level);
        if depth > limits.get().max_ast_nesting_depth {
            return Err(BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::AstDepthLimit,
                "P1121",
                format!("{base}/level"),
            ));
        }
        if owner.is_some_and(|owner| {
            owner.kind == StagingOutlineSourceKind::Heading
                && owner.heading_level != Some(entry.level)
        }) {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/level"),
            ));
        }
        if usize::try_from(entry.outline_id) != Ok(index) {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/outline_id"),
            ));
        }
        while stack.last().is_some_and(|(level, _)| *level >= entry.level) {
            stack.pop();
        }
        let expected_parent = if entry.level == 1 {
            None
        } else {
            stack
                .last()
                .filter(|(level, _)| level.checked_add(1) == Some(entry.level))
                .map(|(_, outline_id)| *outline_id)
        };
        if entry.parent_outline_id != expected_parent
            || (entry.level == 1 && entry.parent_outline_id.is_some())
            || (entry.level > 1 && expected_parent.is_none())
        {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/parent_outline_id"),
            ));
        }
        let owner = owner.ok_or_else(|| {
            BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/source_node_id"),
            )
        })?;
        let requested_kind = match entry.source_kind {
            WireOutlineSourceKind::Heading => StagingOutlineSourceKind::Heading,
            WireOutlineSourceKind::SemanticContainer => StagingOutlineSourceKind::SemanticContainer,
        };
        if requested_kind != owner.kind {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/source_kind"),
            ));
        }
        if previous_source.is_some_and(|prior| prior >= entry.source_node_id)
            || !sources.insert(entry.source_node_id)
        {
            return Err(BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/source_node_id"),
            ));
        }
        previous_source = Some(entry.source_node_id);
        let anchor = owner.anchor.as_deref().ok_or_else(|| {
            BookNavigationSyntaxError::producer(
                BookNavigationSyntaxErrorKind::InvalidOutline,
                format!("{base}/destination"),
            )
        })?;
        let language = languages
            .record(NodeId::new(owner.node_id))
            .map(|record| record.effective_language.clone())
            .ok_or_else(BookNavigationSyntaxError::mismatch)?;
        output.push(StagingOutlineEntry {
            outline_id: entry.outline_id,
            parent_outline_id: entry.parent_outline_id,
            level: entry.level,
            destination,
            label: entry.label.clone(),
            source: StagingOutlineSource {
                kind: owner.kind,
                node_id: NodeId::new(owner.node_id),
                source_span: lower_span(owner.span)?,
                anchor_id: AnchorId::new(anchor.to_owned()).map_err(|_| {
                    BookNavigationSyntaxError::producer(
                        BookNavigationSyntaxErrorKind::InvalidOutline,
                        format!("{base}/destination"),
                    )
                })?,
                heading_level: owner.heading_level,
                semantic_kind: owner.semantic_kind.clone(),
                computed_language: language.to_string(),
            },
        });
        stack.push((entry.level, entry.outline_id));
    }
    let canonical_jcs = encode_outline(
        &output,
        package_sha256,
        limits_sha256,
        semantic_sha256,
        languages.fingerprint,
    );
    Ok(ValidatedOutlineRegistryReceipt {
        entries: output,
        package_sha256,
        limits_sha256,
        semantic_sha256,
        language_sha256: languages.fingerprint,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn validate_navigation_node_limits(
    wire: &typaxis_document_package::WireStagingM4DocumentPackage,
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
) -> Result<(), BookNavigationSyntaxError> {
    if limits.get().max_ast_nesting_depth < 2 {
        return Err(BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::AstDepthLimit,
            "P1121",
            "/outline",
        ));
    }
    let wire_nodes = staging_m4_wire_ast_node_count(wire, limits.get().max_ast_nesting_depth)
        .map_err(|_| BookNavigationSyntaxError::mismatch())?;
    let total_nodes = package
        .math_nodes()
        .iter()
        .try_fold(wire_nodes, |total, math| {
            total.checked_add(math.parsed().ast_node_count())
        })
        .ok_or_else(|| {
            BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::AstNodeLimit,
                "P1120",
                "/document",
            )
        })?;
    if total_nodes > limits.get().max_ast_nodes {
        return Err(BookNavigationSyntaxError::limit(
            BookNavigationSyntaxErrorKind::AstNodeLimit,
            "P1120",
            "/outline/entries",
        ));
    }
    Ok(())
}

fn validate_aggregate_text(
    wire: &typaxis_document_package::WireStagingM4DocumentPackage,
    languages: &ComputedLanguageRegistryReceipt,
    language_charges: &[(Option<String>, String)],
    limits: &ValidatedResourceLimits,
) -> Result<(), BookNavigationSyntaxError> {
    fn charge(
        total: &mut u64,
        bytes: usize,
        pointer: &str,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), BookNavigationSyntaxError> {
        let bytes = u64::try_from(bytes).map_err(|_| {
            BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::TextAggregateLimit,
                "T2101",
                pointer,
            )
        })?;
        *total = total.checked_add(bytes).ok_or_else(|| {
            BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::TextAggregateLimit,
                "T2101",
                pointer,
            )
        })?;
        if *total > limits.get().max_text_bytes {
            return Err(BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::TextAggregateLimit,
                "T2101",
                pointer,
            ));
        }
        Ok(())
    }

    let mut total = 0u64;
    for (index, buffer) in wire.text_buffers().iter().enumerate() {
        charge(
            &mut total,
            buffer.utf8.len(),
            &format!("/text_buffers/{index}/utf8"),
            limits,
        )?;
    }
    charge(
        &mut total,
        usize::try_from(math_speech_bytes(wire.document())).map_err(|_| {
            BookNavigationSyntaxError::limit(
                BookNavigationSyntaxErrorKind::TextAggregateLimit,
                "T2101",
                "/document",
            )
        })?,
        "/document",
        limits,
    )?;

    let metadata = wire.metadata();
    for (pointer, value) in [
        ("/metadata/author", metadata.author.as_deref()),
        ("/metadata/created", metadata.created.as_deref()),
        ("/metadata/identifier", metadata.identifier.as_deref()),
    ] {
        if let Some(value) = value {
            charge(&mut total, value.len(), pointer, limits)?;
        }
    }
    for (index, keyword) in metadata.keywords.iter().enumerate() {
        charge(
            &mut total,
            keyword.len(),
            &format!("/metadata/keywords/{index}"),
            limits,
        )?;
    }
    for (pointer, value) in [
        ("/metadata/modified", metadata.modified.as_deref()),
        ("/metadata/subject", metadata.subject.as_deref()),
        ("/metadata/title", metadata.title.as_deref()),
    ] {
        if let Some(value) = value {
            charge(&mut total, value.len(), pointer, limits)?;
        }
    }

    if language_charges.len() != languages.records.len() {
        return Err(BookNavigationSyntaxError::mismatch());
    }
    for ((raw, pointer), record) in language_charges.iter().zip(&languages.records) {
        if let Some(raw) = raw
            .as_ref()
            .filter(|raw| raw.as_str() != record.effective_language.as_ref())
        {
            charge(&mut total, raw.len(), pointer, limits)?;
        }
        charge(&mut total, record.effective_language.len(), pointer, limits)?;
    }
    for (index, entry) in wire.outline().entries.iter().enumerate() {
        charge(
            &mut total,
            entry.label.len(),
            &format!("/outline/entries/{index}/label"),
            limits,
        )?;
    }
    Ok(())
}

fn math_speech_bytes(document: &WireStagingM4Document) -> u64 {
    fn inlines(values: &[WireStagingM4Inline]) -> u64 {
        values
            .iter()
            .map(|value| match value {
                WireStagingM4Inline::InlineMath { speech, .. } => speech.len() as u64,
                WireStagingM4Inline::Emphasis { children, .. }
                | WireStagingM4Inline::Strong { children, .. }
                | WireStagingM4Inline::Link { children, .. } => inlines(children),
                _ => 0,
            })
            .sum()
    }
    fn blocks(values: &[WireStagingM4Block]) -> u64 {
        values
            .iter()
            .map(|value| match value {
                WireStagingM4Block::Paragraph { children, .. }
                | WireStagingM4Block::Heading { children, .. } => inlines(children),
                WireStagingM4Block::List { items, .. } => {
                    items.iter().map(|item| blocks(&item.blocks)).sum()
                }
                WireStagingM4Block::Table { head, body, .. } => head
                    .iter()
                    .chain(body)
                    .flat_map(|row| &row.cells)
                    .map(|cell| blocks(&cell.blocks))
                    .sum(),
                WireStagingM4Block::Figure { caption, .. }
                | WireStagingM4Block::VectorFigure { caption, .. }
                | WireStagingM4Block::SemanticContainer {
                    blocks: caption, ..
                } => blocks(caption),
                WireStagingM4Block::DisplayMath { speech, .. } => speech.len() as u64,
                WireStagingM4Block::PageBreak { .. }
                | WireStagingM4Block::MathVectorBlock { .. } => 0,
            })
            .sum()
    }
    blocks(&document.blocks)
        + document
            .footnotes
            .iter()
            .map(|footnote| blocks(&footnote.blocks))
            .sum::<u64>()
}

fn raw_block_span(block: &WireStagingM4Block) -> WireStagingSourceSpan {
    match block {
        WireStagingM4Block::Paragraph { span, .. }
        | WireStagingM4Block::Heading { span, .. }
        | WireStagingM4Block::List { span, .. }
        | WireStagingM4Block::Table { span, .. }
        | WireStagingM4Block::Figure { span, .. }
        | WireStagingM4Block::PageBreak { span, .. }
        | WireStagingM4Block::DisplayMath { span, .. }
        | WireStagingM4Block::VectorFigure { span, .. }
        | WireStagingM4Block::MathVectorBlock { span, .. }
        | WireStagingM4Block::SemanticContainer { span, .. } => *span,
    }
}

fn lower_span(value: WireStagingSourceSpan) -> Result<SourceSpan, BookNavigationSyntaxError> {
    SourceSpan::new(
        SourceId::new(value.source_id),
        Utf8ByteOffset::new(value.start_byte),
        Utf8ByteOffset::new(value.end_byte),
    )
    .ok_or_else(BookNavigationSyntaxError::mismatch)
}

fn is_unicode_16_white_space(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

fn encode_metadata(
    value: &StagingDocumentMetadata,
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, DOCUMENT_METADATA_ALGORITHM);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, limits_sha256);
    output.push_str(",\"metadata\":{\"author\":");
    push_nullable(&mut output, value.author.as_deref());
    output.push_str(",\"created\":");
    push_nullable(&mut output, value.created.as_deref());
    output.push_str(",\"identifier\":");
    push_nullable(&mut output, value.identifier.as_deref());
    output.push_str(",\"keywords\":[");
    for (index, keyword) in value.keywords.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, keyword);
    }
    output.push_str("],\"modified\":");
    push_nullable(&mut output, value.modified.as_deref());
    output.push_str(",\"subject\":");
    push_nullable(&mut output, value.subject.as_deref());
    output.push_str(",\"title\":");
    push_nullable(&mut output, value.title.as_deref());
    output.push_str("},\"package_sha256\":");
    push_hash(&mut output, package_sha256);
    output.push('}');
    output
}

fn encode_languages(
    document_language: &str,
    records: &[StagingComputedLanguageRecord],
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, COMPUTED_LANGUAGE_REGISTRY_ALGORITHM);
    output.push_str(",\"document_language\":");
    push_jcs_string(&mut output, document_language);
    output.push_str(",\"language_algorithm\":");
    push_jcs_string(&mut output, BCP47_LANGUAGE_ALGORITHM);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, limits_sha256);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package_sha256);
    output.push_str(",\"records\":[");
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"effective_language\":");
        push_jcs_string(&mut output, &record.effective_language);
        output.push_str(",\"explicit_language\":");
        push_nullable(&mut output, record.explicit_language.as_deref());
        output.push_str(",\"logical_parent_node_id\":");
        if let Some(parent) = record.logical_parent_node_id {
            output.push_str(&parent.get().to_string());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"node_id\":");
        output.push_str(&record.node_id.get().to_string());
        output.push_str(",\"node_kind\":");
        push_jcs_string(&mut output, record.node_kind.as_str());
        output.push_str(",\"source_span\":");
        push_span(&mut output, record.source_span);
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_outline(
    entries: &[StagingOutlineEntry],
    package_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    language_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, OUTLINE_REGISTRY_ALGORITHM);
    output.push_str(",\"entries\":[");
    for (index, entry) in entries.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"destination\":");
        push_jcs_string(&mut output, entry.destination.as_str());
        output.push_str(",\"label\":");
        push_jcs_string(&mut output, &entry.label);
        output.push_str(",\"level\":");
        output.push_str(&entry.level.to_string());
        output.push_str(",\"outline_id\":");
        output.push_str(&entry.outline_id.to_string());
        output.push_str(",\"parent_outline_id\":");
        if let Some(parent) = entry.parent_outline_id {
            output.push_str(&parent.to_string());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"source\":{\"anchor_id\":");
        push_jcs_string(&mut output, entry.source.anchor_id.as_str());
        output.push_str(",\"computed_language\":");
        push_jcs_string(&mut output, &entry.source.computed_language);
        output.push_str(",\"heading_level\":");
        if let Some(level) = entry.source.heading_level {
            output.push_str(&level.to_string());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, entry.source.kind.as_str());
        output.push_str(",\"node_id\":");
        output.push_str(&entry.source.node_id.get().to_string());
        output.push_str(",\"semantic_kind\":");
        push_nullable(&mut output, entry.source.semantic_kind.as_deref());
        output.push_str(",\"source_span\":");
        push_span(&mut output, Some(entry.source.source_span));
        output.push_str("}}");
    }
    output.push_str("],\"language_sha256\":");
    push_hash(&mut output, language_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, limits_sha256);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package_sha256);
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, semantic_sha256);
    output.push('}');
    output
}

fn encode_profile_view(value: &StagingBookNavigationProfileView) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, BOOK_NAVIGATION_PROFILE_VIEW_ALGORITHM);
    for (name, hash) in [
        ("language_sha256", value.language_sha256),
        ("limits_sha256", value.limits_sha256),
        ("metadata_sha256", value.metadata_sha256),
        ("outline_sha256", value.outline_sha256),
        ("package_sha256", value.package_sha256),
        ("semantic_sha256", value.semantic_sha256),
    ] {
        output.push(',');
        push_jcs_string(&mut output, name);
        output.push(':');
        push_hash(&mut output, hash);
    }
    output.push('}');
    output
}

fn limits_fingerprint(limits: &ValidatedResourceLimits) -> [u8; 32] {
    let value = limits.get();
    let mut output = String::from("{");
    macro_rules! fields {
        ($(($name:literal, $value:expr)),+ $(,)?) => {{
            let mut first = true;
            $(
                if !first { output.push(','); }
                first = false;
                output.push_str(concat!("\"", $name, "\":"));
                output.push_str(&$value.to_string());
            )+
            let _ = first;
        }};
    }
    fields!(
        ("max_ast_nesting_depth", value.max_ast_nesting_depth),
        ("max_ast_nodes", value.max_ast_nodes),
        ("max_cids_per_font", value.max_cids_per_font),
        (
            "max_column_balance_candidates",
            value.max_column_balance_candidates
        ),
        ("max_decoded_image_bytes", value.max_decoded_image_bytes),
        (
            "max_document_package_bytes",
            value.max_document_package_bytes
        ),
        ("max_float_carry_pages", value.max_float_carry_pages),
        ("max_float_queue", value.max_float_queue),
        ("max_font_bytes", value.max_font_bytes),
        ("max_fonts", value.max_fonts),
        (
            "max_footnote_reflows_per_page",
            value.max_footnote_reflows_per_page
        ),
        ("max_fragments", value.max_fragments),
        ("max_image_bytes", value.max_image_bytes),
        ("max_image_pixels", value.max_image_pixels),
        ("max_images", value.max_images),
        ("max_include_depth", value.max_include_depth),
        ("max_include_files", value.max_include_files),
        ("max_input_bytes", value.max_input_bytes),
        ("max_json_nesting_depth", value.max_json_nesting_depth),
        ("max_layout_passes", value.max_layout_passes),
        ("max_line_reshape_passes", value.max_line_reshape_passes),
        ("max_output_bytes", value.max_output_bytes),
        ("max_page_break_lookback", value.max_page_break_lookback),
        ("max_pages", value.max_pages),
        ("max_pdf_objects", value.max_pdf_objects),
        ("max_resource_bytes", value.max_resource_bytes),
        ("max_shaping_context_bytes", value.max_shaping_context_bytes),
        ("max_source_bytes", value.max_source_bytes),
        ("max_spool_bytes", value.max_spool_bytes),
        ("max_style_rules", value.max_style_rules),
        ("max_text_buffer_bytes", value.max_text_buffer_bytes),
        ("max_text_bytes", value.max_text_bytes),
        ("max_uri_bytes", value.max_uri_bytes),
    );
    output.push('}');
    sha256(output.as_bytes())
}

fn push_nullable(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

fn push_span(output: &mut String, value: Option<SourceSpan>) {
    if let Some(value) = value {
        output.push_str("{\"end_byte\":");
        output.push_str(&value.end_byte().get().to_string());
        output.push_str(",\"source_id\":");
        output.push_str(&value.source_id().get().to_string());
        output.push_str(",\"start_byte\":");
        output.push_str(&value.start_byte().get().to_string());
        output.push('}');
    } else {
        output.push_str("null");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_document_package::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireDocumentOutline,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"
    ));
    const MATH_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json"
    ));

    fn package() -> (ValidatedStagingSemanticPackage, ValidatedResourceLimits) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        (package, limits)
    }

    fn mutated_outline_package(
        mutate: impl FnOnce(&mut WireDocumentOutline),
    ) -> (ValidatedStagingSemanticPackage, ValidatedResourceLimits) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        let metadata = wire.metadata().clone();
        let mut outline = wire.outline().clone();
        mutate(&mut outline);
        wire.replace_book_navigation(metadata, outline);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                encoded.as_bytes(),
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        (package, limits)
    }

    #[test]
    fn book_navigation_validates_metadata_language_inheritance_and_outline() {
        let (package, limits) = package();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        assert_eq!(navigation.languages().document_language(), "en-US");
        assert_eq!(navigation.outline().entries().len(), 3);
        assert_eq!(
            navigation.outline().entries()[1].source.computed_language,
            "fr-Latn-FR"
        );
        let document = navigation.languages().record(NodeId::new(0)).unwrap();
        let container = navigation.languages().record(NodeId::new(1)).unwrap();
        let heading = navigation.languages().record(NodeId::new(2)).unwrap();
        let heading_text = navigation.languages().record(NodeId::new(3)).unwrap();
        assert!(Arc::ptr_eq(
            &document.effective_language,
            &container.effective_language
        ));
        assert!(Arc::ptr_eq(
            heading.explicit_language.as_ref().unwrap(),
            &heading.effective_language
        ));
        assert!(Arc::ptr_eq(
            &heading.effective_language,
            &heading_text.effective_language
        ));
        navigation.verify(&package, &limits).unwrap();
    }

    #[test]
    fn book_navigation_page_region_content_inherits_document_language() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes =
            typaxis_document_package::staging_book_navigation_page_region_fixture(FIXTURE).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let block = navigation.languages().record(NodeId::new(11)).unwrap();
        let text = navigation.languages().record(NodeId::new(12)).unwrap();
        assert_eq!(block.logical_parent_node_id, Some(NodeId::new(0)));
        assert_eq!(block.effective_language.as_ref(), "en-US");
        assert_eq!(text.logical_parent_node_id, Some(NodeId::new(11)));
        assert_eq!(text.effective_language.as_ref(), "en-US");
        assert!(navigation.languages().record(NodeId::new(10)).is_none());
    }

    #[test]
    fn book_navigation_ast_limit_combines_page_regions_and_navigation_nodes() {
        let default_limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes =
            typaxis_document_package::staging_book_navigation_page_region_fixture(FIXTURE).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&default_limits))
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &default_limits)
            .unwrap();
        let wire = package.checked_wire().unwrap();
        let exact =
            staging_m4_wire_ast_node_count(wire, default_limits.get().max_ast_nesting_depth)
                .unwrap()
                + package
                    .math_nodes()
                    .iter()
                    .map(|math| math.parsed().ast_node_count())
                    .sum::<u64>();

        let parse = |maximum| {
            let raw = ResourceLimits {
                max_ast_nodes: maximum,
                ..ResourceLimits::default()
            };
            let limits = ValidatedResourceLimits::new(raw).unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
                .unwrap();
            let package = crate::StagingSemanticPackageParser::new()
                .parse(decoded, &limits)
                .unwrap();
            (package, limits)
        };
        let (exact_package, exact_limits) = parse(exact);
        validate_staging_book_navigation(&exact_package, &exact_limits).unwrap();
        let over_limits = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: exact - 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&over_limits))
            .is_err());
    }

    #[test]
    fn book_navigation_ast_limit_combines_wire_and_parsed_math_nodes() {
        let default_limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                MATH_FIXTURE,
                &DocumentPackageDecodePolicy::new(&default_limits),
            )
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &default_limits)
            .unwrap();
        let wire = package.checked_wire().unwrap();
        let exact =
            staging_m4_wire_ast_node_count(wire, default_limits.get().max_ast_nesting_depth)
                .unwrap()
                + package
                    .math_nodes()
                    .iter()
                    .map(|math| math.parsed().ast_node_count())
                    .sum::<u64>();
        let parse = |maximum| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_ast_nodes: maximum,
                ..ResourceLimits::default()
            })
            .unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(MATH_FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
                .unwrap();
            let package = crate::StagingSemanticPackageParser::new()
                .parse(decoded, &limits)
                .unwrap();
            (package, limits)
        };
        let (exact_package, exact_limits) = parse(exact);
        validate_staging_book_navigation(&exact_package, &exact_limits).unwrap();
        let (over_package, over_limits) = parse(exact - 1);
        assert_eq!(
            validate_staging_book_navigation(&over_package, &over_limits)
                .unwrap_err()
                .kind(),
            BookNavigationSyntaxErrorKind::AstNodeLimit,
        );
    }

    #[test]
    fn book_navigation_bcp47_is_registry_independent_and_canonical() {
        assert_eq!(
            canonicalize_bcp47_language("EN-latn-us-u-CA-gregory-a-foo").unwrap(),
            "en-Latn-US-a-foo-u-ca-gregory"
        );
        assert_eq!(
            canonicalize_bcp47_language("I-KLINGON").unwrap(),
            "i-klingon"
        );
        assert_eq!(
            canonicalize_bcp47_language("SGN-be-fr").unwrap(),
            "sgn-BE-FR"
        );
        let fixed_cap = canonicalize_language_with_limit(
            &format!("x-{}", ["abcdefgh"; 32].join("-")),
            "/language",
            64,
        )
        .unwrap_err();
        assert_eq!(fixed_cap.code(), "P1102");
        let configured_cap = canonicalize_language_with_limit(
            &format!("x-{}", ["abcdefgh"; 8].join("-")),
            "/language",
            64,
        )
        .unwrap_err();
        assert_eq!(configured_cap.code(), "T2100");
        assert!(canonicalize_bcp47_language("en-variant-Variant").is_err());
        assert!(canonicalize_bcp47_language("en-a").is_err());
        assert!(canonicalize_bcp47_language("en_US").is_err());
    }

    #[test]
    fn book_navigation_language_aggregate_charges_distinct_raw_and_each_computed_instance() {
        let (package, default_limits) = package();
        let navigation = validate_staging_book_navigation(&package, &default_limits).unwrap();
        let wire = package.checked_wire().unwrap();
        let mut sites = Vec::new();
        let mut owners = BTreeMap::new();
        let mut anchors = BTreeMap::new();
        collect_document(
            wire.document(),
            wire.advanced_page_masters(),
            &mut sites,
            &mut owners,
            &mut anchors,
        )
        .unwrap();
        let language_charges = sites
            .iter()
            .map(|site| (site.raw.clone(), site.pointer.clone()))
            .collect::<Vec<_>>();
        let metadata = wire.metadata();
        let metadata_bytes = [
            metadata.author.as_deref(),
            metadata.created.as_deref(),
            metadata.identifier.as_deref(),
            metadata.modified.as_deref(),
            metadata.subject.as_deref(),
            metadata.title.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(str::len)
        .sum::<usize>()
            + metadata.keywords.iter().map(String::len).sum::<usize>();
        let language_bytes = language_charges
            .iter()
            .zip(navigation.languages().records())
            .map(|((raw, _), record)| {
                raw.as_deref()
                    .filter(|raw| *raw != record.effective_language.as_ref())
                    .map_or(0, str::len)
                    + record.effective_language.len()
            })
            .sum::<usize>();
        let exact = wire
            .text_buffers()
            .iter()
            .map(|buffer| buffer.utf8.len())
            .sum::<usize>()
            + usize::try_from(math_speech_bytes(wire.document())).unwrap()
            + metadata_bytes
            + language_bytes
            + wire
                .outline()
                .entries
                .iter()
                .map(|entry| entry.label.len())
                .sum::<usize>();

        let limits = |maximum: usize| {
            let raw = ResourceLimits {
                max_text_buffer_bytes: 64,
                max_shaping_context_bytes: 64,
                max_text_bytes: u64::try_from(maximum).unwrap(),
                ..ResourceLimits::default()
            };
            ValidatedResourceLimits::new(raw).unwrap()
        };
        assert!(validate_aggregate_text(
            wire,
            navigation.languages(),
            &language_charges,
            &limits(exact),
        )
        .is_ok());
        assert_eq!(
            validate_aggregate_text(
                wire,
                navigation.languages(),
                &language_charges,
                &limits(exact - 1),
            )
            .unwrap_err()
            .kind(),
            BookNavigationSyntaxErrorKind::TextAggregateLimit,
        );
    }

    #[test]
    fn book_navigation_rejects_bad_hierarchy_and_metadata_without_pdf() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes = typaxis_document_package::staging_book_navigation_wrong_parent_fixture(FIXTURE)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let error = validate_staging_book_navigation(&package, &limits).unwrap_err();
        assert_eq!(error.code(), "P1102");
        assert_eq!(
            error.pointer().as_str(),
            "/outline/entries/1/parent_outline_id"
        );
    }

    #[test]
    fn book_navigation_outline_diagnostics_follow_canonical_member_order() {
        let (package, limits) = mutated_outline_package(|outline| {
            outline.entries[0].destination = "1-invalid".to_owned();
            outline.entries[0].outline_id = 9;
        });
        assert_eq!(
            validate_staging_book_navigation(&package, &limits)
                .unwrap_err()
                .pointer()
                .as_str(),
            "/outline/entries/0/destination"
        );

        let (package, limits) = mutated_outline_package(|outline| {
            outline.entries[0].label = " ".to_owned();
            outline.entries[0].level = 9;
        });
        assert_eq!(
            validate_staging_book_navigation(&package, &limits)
                .unwrap_err()
                .pointer()
                .as_str(),
            "/outline/entries/0/label"
        );
    }
}
