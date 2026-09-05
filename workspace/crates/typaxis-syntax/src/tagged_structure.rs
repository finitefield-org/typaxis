use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{
    push_jcs_string, sha256, M4EffectiveResourceLimits, NodeId, SourceId, SourceSpan, TextSpan,
    Utf8ByteOffset,
};
use typaxis_document::{StagingComputedLanguageChildKindV2, StagingComputedLanguageOwnerKindV2};
use typaxis_document_package::{
    WireStagingM4Block, WireStagingM4Document, WireStagingM4Inline, WireStagingM4ReferenceFormat,
    WireStagingM4TableRow, WireStagingSourceSpan,
};

use crate::{
    BookNavigationSyntaxError, PrecomposedVectorKind, ValidatedStagingBookNavigation,
    ValidatedStagingBookNavigationV2, ValidatedStagingSemanticPackage,
};

pub const STAGING_STRUCTURE_SEMANTIC_INPUT_ALGORITHM: &str = "typaxis.structure-semantic-input/1";
pub const STAGING_ACCESSIBILITY_PROFILE_VIEW_ALGORITHM: &str =
    "typaxis.production-accessibility-profile-view/1";
pub const STAGING_ACCESSIBILITY_AUTHORIZATION_ALGORITHM: &str =
    "typaxis.production-accessibility-authorization/1";
pub const STAGING_ACCESSIBILITY_AUTHORIZATION_ALGORITHM_V2: &str =
    "typaxis.production-accessibility-authorization/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingStructureLanguageBindingV2 {
    record_fingerprint: [u8; 32],
    parent_record_fingerprint: Option<[u8; 32]>,
}

impl StagingStructureLanguageBindingV2 {
    pub const fn record_fingerprint(self) -> [u8; 32] {
        self.record_fingerprint
    }

    pub const fn parent_record_fingerprint(self) -> Option<[u8; 32]> {
        self.parent_record_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingStructureEquationNumberV2 {
    parent_owner: NodeId,
    text_span: TextSpan,
    text_buffer_sha256: [u8; 32],
    exact_text: String,
    exact_text_sha256: [u8; 32],
}

impl StagingStructureEquationNumberV2 {
    pub const fn parent_owner(&self) -> NodeId {
        self.parent_owner
    }

    pub const fn text_span(&self) -> TextSpan {
        self.text_span
    }

    pub const fn text_buffer_sha256(&self) -> [u8; 32] {
        self.text_buffer_sha256
    }

    pub fn exact_text(&self) -> &str {
        &self.exact_text
    }

    pub const fn exact_text_sha256(&self) -> [u8; 32] {
        self.exact_text_sha256
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingStructureTableSection {
    Head,
    Body,
}

impl StagingStructureTableSection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Body => "body",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingStructureSemanticKind {
    Document,
    SemanticContainer {
        semantic_kind: String,
    },
    Paragraph {
        has_real_content: bool,
    },
    Heading {
        level: u8,
        has_real_content: bool,
    },
    List {
        ordered: bool,
    },
    ListItem {
        marker: String,
    },
    Table {
        head_rows: u32,
        body_rows: u32,
    },
    TableRow {
        section: StagingStructureTableSection,
        row_ordinal: u32,
    },
    TableCell {
        section: StagingStructureTableSection,
        row_ordinal: u32,
        column_ordinal: u32,
        colspan: u16,
        rowspan: u16,
        header_node_ids: Vec<NodeId>,
        has_real_content: bool,
    },
    Figure {
        alternative: String,
        has_caption: bool,
    },
    PageBreak,
    DisplayMath {
        alternative: String,
    },
    FootnoteDefinition {
        footnote_id: String,
        marker: String,
        reference_node_ids: Vec<NodeId>,
        placement_valid: bool,
    },
    Text {
        text: String,
    },
    InlineMath {
        alternative: String,
    },
    InlineVector {
        alternative: String,
        authored_actual_text: Option<String>,
        metrics_fingerprint: [u8; 32],
    },
    MathVector {
        alternative: String,
        resolved_actual_text: String,
        metrics_fingerprint: [u8; 32],
    },
    VectorFigure {
        alternative: String,
        has_caption: bool,
        metrics_fingerprint: [u8; 32],
    },
    MathVectorBlock {
        alternative: String,
        resolved_actual_text: String,
        metrics_fingerprint: [u8; 32],
        equation_number_node_id: Option<NodeId>,
    },
    EquationNumber {
        binding: StagingStructureEquationNumberV2,
    },
    Emphasis,
    Strong,
    Link {
        accessible_name: String,
    },
    Anchor,
    Reference {
        label: String,
    },
    FootnoteReference {
        footnote_id: String,
        marker: String,
        placement_valid: bool,
    },
    SoftBreak,
    HardBreak,
}

impl StagingStructureSemanticKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::SemanticContainer { .. } => "semantic_container",
            Self::Paragraph { .. } => "paragraph",
            Self::Heading { .. } => "heading",
            Self::List { .. } => "list",
            Self::ListItem { .. } => "list_item",
            Self::Table { .. } => "table",
            Self::TableRow { .. } => "table_row",
            Self::TableCell { .. } => "table_cell",
            Self::Figure { .. } => "figure",
            Self::PageBreak => "page_break",
            Self::DisplayMath { .. } => "display_math",
            Self::FootnoteDefinition { .. } => "footnote_definition",
            Self::Text { .. } => "text",
            Self::InlineMath { .. } => "inline_math",
            Self::InlineVector { .. } => "inline_vector",
            Self::MathVector { .. } => "math_vector",
            Self::VectorFigure { .. } => "vector_figure",
            Self::MathVectorBlock { .. } => "math_vector_block",
            Self::EquationNumber { .. } => "equation_number",
            Self::Emphasis => "emphasis",
            Self::Strong => "strong",
            Self::Link { .. } => "link",
            Self::Anchor => "anchor",
            Self::Reference { .. } => "reference",
            Self::FootnoteReference { .. } => "footnote_reference",
            Self::SoftBreak => "soft_break",
            Self::HardBreak => "hard_break",
        }
    }

    pub const fn creates_structure_element(&self) -> bool {
        !matches!(
            self,
            Self::PageBreak | Self::Anchor | Self::SoftBreak | Self::HardBreak
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingStructureSemanticRecord {
    node_id: NodeId,
    parent_node_id: Option<NodeId>,
    insertion_after_node_id: Option<NodeId>,
    source_span: Option<SourceSpan>,
    language: String,
    language_binding_v2: Option<StagingStructureLanguageBindingV2>,
    outline_ids: Vec<u32>,
    kind: StagingStructureSemanticKind,
}

impl StagingStructureSemanticRecord {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn parent_node_id(&self) -> Option<NodeId> {
        self.parent_node_id
    }
    pub const fn insertion_after_node_id(&self) -> Option<NodeId> {
        self.insertion_after_node_id
    }
    pub const fn source_span(&self) -> Option<SourceSpan> {
        self.source_span
    }
    pub fn language(&self) -> &str {
        &self.language
    }
    pub const fn language_binding_v2(&self) -> Option<StagingStructureLanguageBindingV2> {
        self.language_binding_v2
    }
    pub fn outline_ids(&self) -> &[u32] {
        &self.outline_ids
    }
    pub const fn kind(&self) -> &StagingStructureSemanticKind {
        &self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingStructureSemantics {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
    language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    records: Vec<StagingStructureSemanticRecord>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedStagingStructureSemantics {
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
    pub fn records(&self) -> &[StagingStructureSemanticRecord] {
        &self.records
    }
    pub fn record(&self, node_id: NodeId) -> Option<&StagingStructureSemanticRecord> {
        self.records
            .binary_search_by_key(&node_id, StagingStructureSemanticRecord::node_id)
            .ok()
            .map(|index| &self.records[index])
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
    ) -> Result<(), StagingStructureSemanticError> {
        let observed = validate_staging_structure_semantics(package, navigation)?;
        if self != &observed {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        Ok(())
    }
}

/// Version-2 semantic closure. The record vocabulary is shared with `/1`, but
/// only this nominal receipt can contain precomposed-vector and equation-number
/// records or computed-language record fingerprints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingStructureSemanticsV2 {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
    language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    records: Vec<StagingStructureSemanticRecord>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedStagingStructureSemanticsV2 {
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
    pub fn records(&self) -> &[StagingStructureSemanticRecord] {
        &self.records
    }
    pub fn record(&self, node_id: NodeId) -> Option<&StagingStructureSemanticRecord> {
        self.records
            .binary_search_by_key(&node_id, StagingStructureSemanticRecord::node_id)
            .ok()
            .map(|index| &self.records[index])
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigationV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingStructureSemanticError> {
        let observed = validate_staging_structure_semantics_v2(package, navigation, limits)?;
        if self != &observed {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingStructureSemanticError {
    NavigationMismatch,
    InvalidSemanticTree,
    InvalidTextBinding,
    InvalidTableGrid,
    AllocationFailure,
    ReceiptMismatch,
    PrecomposedVectorStaging(NodeId),
}

impl std::fmt::Display for StagingStructureSemanticError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NavigationMismatch => {
                formatter.write_str("I9190: structure semantic navigation mismatch")
            }
            Self::InvalidSemanticTree => {
                formatter.write_str("I9190: structure semantic tree mismatch")
            }
            Self::InvalidTextBinding => {
                formatter.write_str("I9190: structure semantic text binding mismatch")
            }
            Self::InvalidTableGrid => {
                formatter.write_str("I9190: structure semantic table grid mismatch")
            }
            Self::AllocationFailure => {
                formatter.write_str("P1120: structure semantic allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: structure semantic receipt mismatch")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires tagged-structure /2",
                owner.get()
            ),
        }
    }
}

impl std::error::Error for StagingStructureSemanticError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAccessibilityProfileView {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    structure_semantics_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
    language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingAccessibilityProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        semantics: &ValidatedStagingStructureSemantics,
    ) -> Result<Self, StagingStructureSemanticError> {
        semantics.verify(package, navigation)?;
        let mut value = Self {
            package_sha256: semantics.package_sha256,
            semantic_sha256: semantics.semantic_sha256,
            structure_semantics_sha256: semantics.fingerprint,
            metadata_sha256: semantics.metadata_sha256,
            language_sha256: semantics.language_sha256,
            outline_sha256: semantics.outline_sha256,
            limits_sha256: semantics.limits_sha256,
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
    pub const fn structure_semantics_sha256(&self) -> [u8; 32] {
        self.structure_semantics_sha256
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAccessibilityProfileAuthorization {
    view: StagingAccessibilityProfileView,
    profile_receipt_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingAccessibilityProfileAuthorization {
    #[doc(hidden)]
    pub fn bind_profile_receipt(
        view: StagingAccessibilityProfileView,
        profile_receipt_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        semantics: &ValidatedStagingStructureSemantics,
    ) -> Result<Self, StagingStructureSemanticError> {
        let expected = StagingAccessibilityProfileView::new(package, navigation, semantics)?;
        if view != expected || profile_receipt_fingerprint == [0; 32] {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        let canonical_jcs = encode_authorization(&view, profile_receipt_fingerprint);
        Ok(Self {
            view,
            profile_receipt_fingerprint,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn view(&self) -> &StagingAccessibilityProfileView {
        &self.view
    }
    pub const fn profile_receipt_fingerprint(&self) -> [u8; 32] {
        self.profile_receipt_fingerprint
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
        semantics: &ValidatedStagingStructureSemantics,
    ) -> Result<(), StagingStructureSemanticError> {
        let expected = StagingAccessibilityProfileView::new(package, navigation, semantics)?;
        let canonical = encode_authorization(&expected, self.profile_receipt_fingerprint);
        if self.view != expected
            || self.profile_receipt_fingerprint == [0; 32]
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAccessibilityProfileViewV2 {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    structure_semantics_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
    language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingAccessibilityProfileViewV2 {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigationV2,
        semantics: &ValidatedStagingStructureSemanticsV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingStructureSemanticError> {
        semantics.verify(package, navigation, limits)?;
        let mut value = Self {
            package_sha256: semantics.package_sha256,
            semantic_sha256: semantics.semantic_sha256,
            structure_semantics_sha256: semantics.fingerprint,
            metadata_sha256: semantics.metadata_sha256,
            language_sha256: semantics.language_sha256,
            outline_sha256: semantics.outline_sha256,
            limits_sha256: semantics.limits_sha256,
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        value.canonical_jcs = encode_profile_view_v2(&value);
        value.fingerprint = sha256(value.canonical_jcs.as_bytes());
        Ok(value)
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn structure_semantics_sha256(&self) -> [u8; 32] {
        self.structure_semantics_sha256
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAccessibilityProfileAuthorizationV2 {
    view: StagingAccessibilityProfileViewV2,
    profile_receipt_fingerprint: [u8; 32],
    book_navigation_profile_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingAccessibilityProfileAuthorizationV2 {
    #[doc(hidden)]
    pub fn bind_profile_receipt(
        view: StagingAccessibilityProfileViewV2,
        profile_receipt_fingerprint: [u8; 32],
        book_navigation_profile_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigationV2,
        semantics: &ValidatedStagingStructureSemanticsV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingStructureSemanticError> {
        let expected =
            StagingAccessibilityProfileViewV2::new(package, navigation, semantics, limits)?;
        if view != expected
            || profile_receipt_fingerprint == [0; 32]
            || book_navigation_profile_fingerprint == [0; 32]
        {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        let canonical_jcs = encode_authorization_v2(
            &view,
            profile_receipt_fingerprint,
            book_navigation_profile_fingerprint,
        );
        Ok(Self {
            view,
            profile_receipt_fingerprint,
            book_navigation_profile_fingerprint,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn view(&self) -> &StagingAccessibilityProfileViewV2 {
        &self.view
    }
    pub const fn profile_receipt_fingerprint(&self) -> [u8; 32] {
        self.profile_receipt_fingerprint
    }
    pub const fn book_navigation_profile_fingerprint(&self) -> [u8; 32] {
        self.book_navigation_profile_fingerprint
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
        navigation: &ValidatedStagingBookNavigationV2,
        semantics: &ValidatedStagingStructureSemanticsV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingStructureSemanticError> {
        let expected =
            StagingAccessibilityProfileViewV2::new(package, navigation, semantics, limits)?;
        let canonical = encode_authorization_v2(
            &expected,
            self.profile_receipt_fingerprint,
            self.book_navigation_profile_fingerprint,
        );
        if self.view != expected
            || self.profile_receipt_fingerprint == [0; 32]
            || self.book_navigation_profile_fingerprint == [0; 32]
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone)]
struct FootnoteReferenceSite {
    node_id: NodeId,
    paragraph_owner: Option<NodeId>,
    direct_paragraph_branch: Option<NodeId>,
    placement_valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StructureSemanticGeneration {
    V1,
    V2,
}

#[derive(Clone, Copy)]
enum StructureSemanticNavigation<'a> {
    V1(&'a ValidatedStagingBookNavigation),
    V2(&'a ValidatedStagingBookNavigationV2),
}

struct SemanticCollector<'a> {
    package: &'a ValidatedStagingSemanticPackage,
    navigation: StructureSemanticNavigation<'a>,
    generation: StructureSemanticGeneration,
    text_buffers: BTreeMap<u32, &'a str>,
    footnote_markers: BTreeMap<String, String>,
    records: Vec<StagingStructureSemanticRecord>,
    footnote_references: BTreeMap<String, Vec<FootnoteReferenceSite>>,
}

pub fn validate_staging_structure_semantics(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
) -> Result<ValidatedStagingStructureSemantics, StagingStructureSemanticError> {
    navigation
        .verify(package, package.limits())
        .map_err(map_navigation_error)?;
    let wire = package
        .checked_wire()
        .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?;
    let text_buffers = wire
        .text_buffers()
        .iter()
        .map(|buffer| (buffer.text_id, buffer.utf8.as_str()))
        .collect::<BTreeMap<_, _>>();
    let footnote_markers = wire
        .document()
        .footnotes
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            let ordinal = index
                .checked_add(1)
                .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?;
            Ok((definition.footnote_id.clone(), ordinal.to_string()))
        })
        .collect::<Result<BTreeMap<_, _>, StagingStructureSemanticError>>()?;
    if footnote_markers.len() != wire.document().footnotes.len() {
        return Err(StagingStructureSemanticError::InvalidSemanticTree);
    }
    let mut collector = SemanticCollector {
        package,
        navigation: StructureSemanticNavigation::V1(navigation),
        generation: StructureSemanticGeneration::V1,
        text_buffers,
        footnote_markers,
        records: Vec::new(),
        footnote_references: BTreeMap::new(),
    };
    collector.push_record(
        wire.document().node_id,
        None,
        None,
        None,
        navigation.languages().document_language().to_owned(),
        StagingStructureSemanticKind::Document,
    )?;
    collector.blocks(
        &wire.document().blocks,
        NodeId::new(wire.document().node_id),
        navigation.languages().document_language(),
        true,
    )?;
    collector.footnotes(wire.document())?;
    if collector.records.len()
        != usize::try_from(
            collector
                .records
                .last()
                .map_or(0, |value| value.node_id.get().saturating_add(1)),
        )
        .map_err(|_| StagingStructureSemanticError::InvalidSemanticTree)?
    {
        return Err(StagingStructureSemanticError::InvalidSemanticTree);
    }
    let outline_by_owner = navigation.outline().entries().iter().fold(
        BTreeMap::<NodeId, Vec<u32>>::new(),
        |mut output, entry| {
            output
                .entry(entry.source.node_id)
                .or_default()
                .push(entry.outline_id);
            output
        },
    );
    for record in &mut collector.records {
        record.outline_ids = outline_by_owner
            .get(&record.node_id)
            .cloned()
            .unwrap_or_default();
    }
    let canonical_jcs = encode_semantics(
        package,
        navigation,
        &collector.records,
        navigation.languages().limits_sha256(),
    );
    Ok(ValidatedStagingStructureSemantics {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_sha256: package.semantic_fingerprint(),
        metadata_sha256: navigation.metadata().fingerprint(),
        language_sha256: navigation.languages().fingerprint(),
        outline_sha256: navigation.outline().fingerprint(),
        limits_sha256: navigation.languages().limits_sha256(),
        records: collector.records,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

pub fn validate_staging_structure_semantics_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    limits: &M4EffectiveResourceLimits,
) -> Result<ValidatedStagingStructureSemanticsV2, StagingStructureSemanticError> {
    if package.limits() != limits.base() {
        return Err(StagingStructureSemanticError::ReceiptMismatch);
    }
    navigation
        .verify(package, limits)
        .map_err(map_navigation_error)?;
    let wire = package
        .checked_wire()
        .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?;
    let text_buffers = wire
        .text_buffers()
        .iter()
        .map(|buffer| (buffer.text_id, buffer.utf8.as_str()))
        .collect::<BTreeMap<_, _>>();
    let footnote_markers = wire
        .document()
        .footnotes
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            let ordinal = index
                .checked_add(1)
                .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?;
            Ok((definition.footnote_id.clone(), ordinal.to_string()))
        })
        .collect::<Result<BTreeMap<_, _>, StagingStructureSemanticError>>()?;
    if footnote_markers.len() != wire.document().footnotes.len() {
        return Err(StagingStructureSemanticError::InvalidSemanticTree);
    }
    let mut collector = SemanticCollector {
        package,
        navigation: StructureSemanticNavigation::V2(navigation),
        generation: StructureSemanticGeneration::V2,
        text_buffers,
        footnote_markers,
        records: Vec::new(),
        footnote_references: BTreeMap::new(),
    };
    collector.push_record(
        wire.document().node_id,
        None,
        None,
        None,
        navigation.languages().document_language().to_owned(),
        StagingStructureSemanticKind::Document,
    )?;
    collector.blocks(
        &wire.document().blocks,
        NodeId::new(wire.document().node_id),
        navigation.languages().document_language(),
        true,
    )?;
    collector.footnotes(wire.document())?;
    if collector.records.len()
        != usize::try_from(
            collector
                .records
                .last()
                .map_or(0, |value| value.node_id.get().saturating_add(1)),
        )
        .map_err(|_| StagingStructureSemanticError::InvalidSemanticTree)?
    {
        return Err(StagingStructureSemanticError::InvalidSemanticTree);
    }
    let bound_language_records = collector
        .records
        .iter()
        .filter(|record| record.language_binding_v2.is_some())
        .count();
    if bound_language_records
        != navigation
            .languages()
            .records()
            .len()
            .checked_add(navigation.languages().child_records().len())
            .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?
        || collector
            .records
            .iter()
            .filter(|record| {
                matches!(
                    record.kind,
                    StagingStructureSemanticKind::InlineVector { .. }
                        | StagingStructureSemanticKind::MathVector { .. }
                        | StagingStructureSemanticKind::VectorFigure { .. }
                        | StagingStructureSemanticKind::MathVectorBlock { .. }
                )
            })
            .count()
            != package.precomposed_vector_metrics().len()
    {
        return Err(StagingStructureSemanticError::NavigationMismatch);
    }
    let outline_by_owner = navigation.outline().entries().iter().fold(
        BTreeMap::<NodeId, Vec<u32>>::new(),
        |mut output, entry| {
            output
                .entry(entry.source.node_id)
                .or_default()
                .push(entry.outline_id);
            output
        },
    );
    for record in &mut collector.records {
        record.outline_ids = outline_by_owner
            .get(&record.node_id)
            .cloned()
            .unwrap_or_default();
    }
    let canonical_jcs = encode_semantics_v2(
        package,
        navigation,
        &collector.records,
        limits.fingerprint(),
    );
    Ok(ValidatedStagingStructureSemanticsV2 {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_sha256: package.semantic_fingerprint(),
        metadata_sha256: navigation.metadata().fingerprint(),
        language_sha256: navigation.languages().fingerprint(),
        outline_sha256: navigation.outline().fingerprint(),
        limits_sha256: limits.fingerprint(),
        records: collector.records,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn map_navigation_error(_: BookNavigationSyntaxError) -> StagingStructureSemanticError {
    StagingStructureSemanticError::NavigationMismatch
}

fn semantic_language_owner_kind_v2(
    kind: &StagingStructureSemanticKind,
) -> Option<StagingComputedLanguageOwnerKindV2> {
    Some(match kind {
        StagingStructureSemanticKind::Document => StagingComputedLanguageOwnerKindV2::Document,
        StagingStructureSemanticKind::SemanticContainer { .. } => {
            StagingComputedLanguageOwnerKindV2::SemanticContainer
        }
        StagingStructureSemanticKind::Paragraph { .. } => {
            StagingComputedLanguageOwnerKindV2::Paragraph
        }
        StagingStructureSemanticKind::Heading { .. } => StagingComputedLanguageOwnerKindV2::Heading,
        StagingStructureSemanticKind::List { .. } => StagingComputedLanguageOwnerKindV2::List,
        StagingStructureSemanticKind::ListItem { .. } => {
            StagingComputedLanguageOwnerKindV2::ListItem
        }
        StagingStructureSemanticKind::Table { .. } => StagingComputedLanguageOwnerKindV2::Table,
        StagingStructureSemanticKind::TableRow { .. } => {
            StagingComputedLanguageOwnerKindV2::TableRow
        }
        StagingStructureSemanticKind::TableCell { .. } => {
            StagingComputedLanguageOwnerKindV2::TableCell
        }
        StagingStructureSemanticKind::Figure { .. } => StagingComputedLanguageOwnerKindV2::Figure,
        StagingStructureSemanticKind::DisplayMath { .. } => {
            StagingComputedLanguageOwnerKindV2::DisplayMath
        }
        StagingStructureSemanticKind::FootnoteDefinition { .. } => {
            StagingComputedLanguageOwnerKindV2::FootnoteDefinition
        }
        StagingStructureSemanticKind::Text { .. } => StagingComputedLanguageOwnerKindV2::Text,
        StagingStructureSemanticKind::InlineMath { .. } => {
            StagingComputedLanguageOwnerKindV2::InlineMath
        }
        StagingStructureSemanticKind::InlineVector { .. } => {
            StagingComputedLanguageOwnerKindV2::InlineVector
        }
        StagingStructureSemanticKind::MathVector { .. } => {
            StagingComputedLanguageOwnerKindV2::MathVector
        }
        StagingStructureSemanticKind::VectorFigure { .. } => {
            StagingComputedLanguageOwnerKindV2::VectorFigure
        }
        StagingStructureSemanticKind::MathVectorBlock { .. } => {
            StagingComputedLanguageOwnerKindV2::MathVectorBlock
        }
        StagingStructureSemanticKind::Emphasis => StagingComputedLanguageOwnerKindV2::Emphasis,
        StagingStructureSemanticKind::Strong => StagingComputedLanguageOwnerKindV2::Strong,
        StagingStructureSemanticKind::Link { .. } => StagingComputedLanguageOwnerKindV2::Link,
        StagingStructureSemanticKind::Reference { .. } => {
            StagingComputedLanguageOwnerKindV2::Reference
        }
        StagingStructureSemanticKind::FootnoteReference { .. } => {
            StagingComputedLanguageOwnerKindV2::FootnoteReference
        }
        StagingStructureSemanticKind::EquationNumber { .. }
        | StagingStructureSemanticKind::PageBreak
        | StagingStructureSemanticKind::Anchor
        | StagingStructureSemanticKind::SoftBreak
        | StagingStructureSemanticKind::HardBreak => return None,
    })
}

impl SemanticCollector<'_> {
    fn push_record(
        &mut self,
        raw_node_id: u32,
        parent_node_id: Option<NodeId>,
        insertion_after_node_id: Option<NodeId>,
        raw_span: Option<WireStagingSourceSpan>,
        language: String,
        kind: StagingStructureSemanticKind,
    ) -> Result<(), StagingStructureSemanticError> {
        if usize::try_from(raw_node_id) != Ok(self.records.len()) {
            return Err(StagingStructureSemanticError::InvalidSemanticTree);
        }
        let source_span = raw_span.map(lower_span).transpose()?;
        let language_binding_v2 =
            self.language_binding_v2(raw_node_id, parent_node_id, source_span, &language, &kind)?;
        self.records
            .try_reserve(1)
            .map_err(|_| StagingStructureSemanticError::AllocationFailure)?;
        self.records.push(StagingStructureSemanticRecord {
            node_id: NodeId::new(raw_node_id),
            parent_node_id,
            insertion_after_node_id,
            source_span,
            language,
            language_binding_v2,
            outline_ids: Vec::new(),
            kind,
        });
        Ok(())
    }

    fn language(
        &self,
        raw_node_id: u32,
        inherited: &str,
    ) -> Result<String, StagingStructureSemanticError> {
        Ok(match self.navigation {
            StructureSemanticNavigation::V1(navigation) => navigation
                .languages()
                .record(NodeId::new(raw_node_id))
                .map_or_else(
                    || inherited.to_owned(),
                    |record| record.effective_language.to_string(),
                ),
            StructureSemanticNavigation::V2(navigation) => navigation
                .languages()
                .record(NodeId::new(raw_node_id))
                .map_or_else(
                    || inherited.to_owned(),
                    |record| record.effective_language.to_string(),
                ),
        })
    }

    fn vector_semantic(
        &self,
        owner: NodeId,
        expected_kind: PrecomposedVectorKind,
    ) -> Result<(String, [u8; 32]), StagingStructureSemanticError> {
        let metrics = self
            .package
            .precomposed_vector_metrics_for(owner)
            .ok_or(StagingStructureSemanticError::ReceiptMismatch)?;
        self.package
            .verify_precomposed_vector_metrics(metrics)
            .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?;
        if metrics.kind() != expected_kind {
            return Err(StagingStructureSemanticError::ReceiptMismatch);
        }
        Ok((
            metrics.alternative().alternative().to_owned(),
            metrics.fingerprint(),
        ))
    }

    fn language_binding_v2(
        &self,
        raw_node_id: u32,
        parent_node_id: Option<NodeId>,
        source_span: Option<SourceSpan>,
        language: &str,
        kind: &StagingStructureSemanticKind,
    ) -> Result<Option<StagingStructureLanguageBindingV2>, StagingStructureSemanticError> {
        let StructureSemanticNavigation::V2(navigation) = self.navigation else {
            return Ok(None);
        };
        if let StagingStructureSemanticKind::EquationNumber { binding } = kind {
            let record = navigation
                .languages()
                .child_record(NodeId::new(raw_node_id))
                .ok_or(StagingStructureSemanticError::NavigationMismatch)?;
            if record.child_kind != StagingComputedLanguageChildKindV2::EquationNumber
                || record.parent_owner_node_id != binding.parent_owner
                || parent_node_id != Some(binding.parent_owner)
                || source_span != Some(record.source_span)
                || record.effective_language.as_ref() != language
            {
                return Err(StagingStructureSemanticError::NavigationMismatch);
            }
            return Ok(Some(StagingStructureLanguageBindingV2 {
                record_fingerprint: record.record_fingerprint,
                parent_record_fingerprint: Some(record.parent_language_record_fingerprint),
            }));
        }
        let Some(expected_kind) = semantic_language_owner_kind_v2(kind) else {
            return Ok(None);
        };
        let record = navigation
            .languages()
            .record(NodeId::new(raw_node_id))
            .ok_or(StagingStructureSemanticError::NavigationMismatch)?;
        // A footnote definition remains a document child in the syntax and
        // language registry, while the tagged structure tree deliberately
        // reparents it beside the last valid reference.  Do not confuse that
        // accessibility placement parent with the language-inheritance parent.
        let language_parent =
            if expected_kind == StagingComputedLanguageOwnerKindV2::FootnoteDefinition {
                Some(NodeId::new(
                    self.package
                        .checked_wire()
                        .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?
                        .document()
                        .node_id,
                ))
            } else {
                parent_node_id
            };
        if record.node_kind != expected_kind
            || record.logical_parent_node_id != language_parent
            || record.source_span != source_span
            || record.effective_language.as_ref() != language
        {
            return Err(StagingStructureSemanticError::NavigationMismatch);
        }
        Ok(Some(StagingStructureLanguageBindingV2 {
            record_fingerprint: record.record_fingerprint,
            parent_record_fingerprint: None,
        }))
    }

    fn blocks(
        &mut self,
        values: &[WireStagingM4Block],
        parent: NodeId,
        inherited_language: &str,
        footnote_reference_placement_valid: bool,
    ) -> Result<(), StagingStructureSemanticError> {
        for value in values {
            let node_id = value.node_id();
            let node = NodeId::new(node_id);
            let language = self.language(node_id, inherited_language)?;
            let span = Some(raw_block_span(value));
            match value {
                WireStagingM4Block::Paragraph { children, .. } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::Paragraph {
                            has_real_content: has_non_whitespace(&inline_text(
                                children,
                                &self.text_buffers,
                                &self.footnote_markers,
                                self.generation,
                            )?),
                        },
                    )?;
                    self.inlines(
                        children,
                        node,
                        node,
                        &language,
                        false,
                        footnote_reference_placement_valid,
                    )?;
                }
                WireStagingM4Block::Heading {
                    level, children, ..
                } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::Heading {
                            level: *level,
                            has_real_content: has_non_whitespace(&inline_text(
                                children,
                                &self.text_buffers,
                                &self.footnote_markers,
                                self.generation,
                            )?),
                        },
                    )?;
                    self.inlines(children, node, node, &language, false, false)?;
                }
                WireStagingM4Block::List {
                    ordered,
                    start,
                    items,
                    ..
                } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::List { ordered: *ordered },
                    )?;
                    for (index, item) in items.iter().enumerate() {
                        let item_node = NodeId::new(item.node_id);
                        let item_language = self.language(item.node_id, &language)?;
                        let marker = if *ordered {
                            let value = start
                                .unwrap_or(1)
                                .checked_add(u32::try_from(index).map_err(|_| {
                                    StagingStructureSemanticError::InvalidSemanticTree
                                })?)
                                .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?;
                            format!("{value}.")
                        } else {
                            "•".to_owned()
                        };
                        self.push_record(
                            item.node_id,
                            Some(node),
                            None,
                            Some(item.span),
                            item_language.clone(),
                            StagingStructureSemanticKind::ListItem { marker },
                        )?;
                        self.blocks(
                            &item.blocks,
                            item_node,
                            &item_language,
                            footnote_reference_placement_valid,
                        )?;
                    }
                }
                WireStagingM4Block::Table {
                    columns,
                    head,
                    body,
                    ..
                } => {
                    let column_count = u16::try_from(columns.len())
                        .map_err(|_| StagingStructureSemanticError::InvalidTableGrid)?;
                    let head_origins = row_origins(head, column_count)?;
                    let body_origins = row_origins(body, column_count)?;
                    let mut header_intervals = Vec::new();
                    for (row_index, row) in head.iter().enumerate() {
                        for (cell_index, cell) in row.cells.iter().enumerate() {
                            let origin = head_origins[row_index][cell_index];
                            header_intervals.push((
                                NodeId::new(cell.node_id),
                                origin,
                                origin.saturating_add(cell.colspan),
                            ));
                        }
                    }
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::Table {
                            head_rows: u32::try_from(head.len())
                                .map_err(|_| StagingStructureSemanticError::InvalidTableGrid)?,
                            body_rows: u32::try_from(body.len())
                                .map_err(|_| StagingStructureSemanticError::InvalidTableGrid)?,
                        },
                    )?;
                    self.table_rows(
                        head,
                        &head_origins,
                        StagingStructureTableSection::Head,
                        node,
                        &language,
                        &header_intervals,
                        footnote_reference_placement_valid,
                    )?;
                    self.table_rows(
                        body,
                        &body_origins,
                        StagingStructureTableSection::Body,
                        node,
                        &language,
                        &header_intervals,
                        footnote_reference_placement_valid,
                    )?;
                }
                WireStagingM4Block::Figure { alt, caption, .. } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::Figure {
                            alternative: alt.clone(),
                            has_caption: !caption.is_empty(),
                        },
                    )?;
                    self.blocks(caption, node, &language, footnote_reference_placement_valid)?;
                }
                WireStagingM4Block::PageBreak { .. } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language,
                        StagingStructureSemanticKind::PageBreak,
                    )?;
                }
                WireStagingM4Block::DisplayMath { speech, .. } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language,
                        StagingStructureSemanticKind::DisplayMath {
                            alternative: speech.clone(),
                        },
                    )?;
                }
                WireStagingM4Block::VectorFigure { caption, .. } => {
                    if self.generation == StructureSemanticGeneration::V1 {
                        return Err(StagingStructureSemanticError::PrecomposedVectorStaging(
                            node,
                        ));
                    }
                    let (alternative, metrics_fingerprint) =
                        self.vector_semantic(node, PrecomposedVectorKind::VectorFigure)?;
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::VectorFigure {
                            alternative,
                            has_caption: !caption.is_empty(),
                            metrics_fingerprint,
                        },
                    )?;
                    self.blocks(caption, node, &language, footnote_reference_placement_valid)?;
                }
                WireStagingM4Block::MathVectorBlock { .. } => {
                    if self.generation == StructureSemanticGeneration::V1 {
                        return Err(StagingStructureSemanticError::PrecomposedVectorStaging(
                            node,
                        ));
                    }
                    let metrics = self
                        .package
                        .precomposed_vector_metrics_for(node)
                        .ok_or(StagingStructureSemanticError::ReceiptMismatch)?;
                    self.package
                        .verify_precomposed_vector_metrics(metrics)
                        .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?;
                    if metrics.kind() != PrecomposedVectorKind::MathVectorBlock {
                        return Err(StagingStructureSemanticError::ReceiptMismatch);
                    }
                    let alternative = metrics.alternative().alternative().to_owned();
                    let resolved_actual_text = metrics
                        .alternative()
                        .resolved_actual_text()
                        .ok_or(StagingStructureSemanticError::ReceiptMismatch)?
                        .to_owned();
                    let metrics_fingerprint = metrics.fingerprint();
                    let equation_number_node_id =
                        metrics.equation_number().map(|number| number.node_id());
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::MathVectorBlock {
                            alternative,
                            resolved_actual_text,
                            metrics_fingerprint,
                            equation_number_node_id,
                        },
                    )?;
                    if let Some(number) = metrics.equation_number() {
                        let exact_text = text_span_value(
                            typaxis_document_package::WireStagingTextSpan {
                                text_id: number.text().text_span().text_id().get(),
                                start_byte: number.text().text_span().start_byte().get(),
                                end_byte: number.text().text_span().end_byte().get(),
                            },
                            &self.text_buffers,
                        )?
                        .to_owned();
                        self.push_record(
                            number.node_id().get(),
                            Some(node),
                            None,
                            Some(WireStagingSourceSpan {
                                source_id: number.span().source_id().get(),
                                start_byte: number.span().start_byte().get(),
                                end_byte: number.span().end_byte().get(),
                            }),
                            language,
                            StagingStructureSemanticKind::EquationNumber {
                                binding: StagingStructureEquationNumberV2 {
                                    parent_owner: node,
                                    text_span: number.text().text_span(),
                                    text_buffer_sha256: number.text().text_buffer_sha256(),
                                    exact_text_sha256: number.text().exact_text_sha256(),
                                    exact_text,
                                },
                            },
                        )?;
                    }
                }
                WireStagingM4Block::SemanticContainer {
                    semantic_kind,
                    blocks,
                    ..
                } => {
                    self.push_record(
                        node_id,
                        Some(parent),
                        None,
                        span,
                        language.clone(),
                        StagingStructureSemanticKind::SemanticContainer {
                            semantic_kind: semantic_kind.as_str().to_owned(),
                        },
                    )?;
                    self.blocks(blocks, node, &language, footnote_reference_placement_valid)?;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn table_rows(
        &mut self,
        rows: &[WireStagingM4TableRow],
        origins: &[Vec<u16>],
        section: StagingStructureTableSection,
        table: NodeId,
        inherited_language: &str,
        header_intervals: &[(NodeId, u16, u16)],
        footnote_reference_placement_valid: bool,
    ) -> Result<(), StagingStructureSemanticError> {
        for (row_index, row) in rows.iter().enumerate() {
            let row_node = NodeId::new(row.node_id);
            let row_language = self.language(row.node_id, inherited_language)?;
            self.push_record(
                row.node_id,
                Some(table),
                None,
                Some(row.span),
                row_language.clone(),
                StagingStructureSemanticKind::TableRow {
                    section,
                    row_ordinal: u32::try_from(row_index)
                        .map_err(|_| StagingStructureSemanticError::InvalidTableGrid)?,
                },
            )?;
            for (cell_index, cell) in row.cells.iter().enumerate() {
                let origin = *origins
                    .get(row_index)
                    .and_then(|values| values.get(cell_index))
                    .ok_or(StagingStructureSemanticError::InvalidTableGrid)?;
                let end = origin
                    .checked_add(cell.colspan)
                    .ok_or(StagingStructureSemanticError::InvalidTableGrid)?;
                let header_node_ids = if section == StagingStructureTableSection::Body {
                    header_intervals
                        .iter()
                        .filter(|(_, start, header_end)| *start < end && origin < *header_end)
                        .map(|(node_id, _, _)| *node_id)
                        .collect()
                } else {
                    Vec::new()
                };
                let cell_language = self.language(cell.node_id, &row_language)?;
                let cell_node = NodeId::new(cell.node_id);
                self.push_record(
                    cell.node_id,
                    Some(row_node),
                    None,
                    Some(cell.span),
                    cell_language.clone(),
                    StagingStructureSemanticKind::TableCell {
                        section,
                        row_ordinal: u32::try_from(row_index)
                            .map_err(|_| StagingStructureSemanticError::InvalidTableGrid)?,
                        column_ordinal: u32::from(origin),
                        colspan: cell.colspan,
                        rowspan: cell.rowspan,
                        header_node_ids,
                        has_real_content: blocks_have_content(
                            &cell.blocks,
                            &self.text_buffers,
                            &self.footnote_markers,
                            self.generation,
                        )?,
                    },
                )?;
                self.blocks(
                    &cell.blocks,
                    cell_node,
                    &cell_language,
                    footnote_reference_placement_valid,
                )?;
            }
        }
        Ok(())
    }

    fn inlines(
        &mut self,
        values: &[WireStagingM4Inline],
        parent: NodeId,
        paragraph_owner: NodeId,
        inherited_language: &str,
        in_link: bool,
        footnote_container_valid: bool,
    ) -> Result<(), StagingStructureSemanticError> {
        for value in values {
            let direct_branch = NodeId::new(value.node_id());
            self.inline(
                value,
                parent,
                paragraph_owner,
                direct_branch,
                inherited_language,
                in_link,
                footnote_container_valid,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn inline(
        &mut self,
        value: &WireStagingM4Inline,
        parent: NodeId,
        paragraph_owner: NodeId,
        direct_paragraph_branch: NodeId,
        inherited_language: &str,
        in_link: bool,
        footnote_container_valid: bool,
    ) -> Result<(), StagingStructureSemanticError> {
        let raw_node_id = value.node_id();
        let node = NodeId::new(raw_node_id);
        let language = self.language(raw_node_id, inherited_language)?;
        let span = Some(value.span());
        match value {
            WireStagingM4Inline::Text { text_span, .. } => self.push_record(
                raw_node_id,
                Some(parent),
                None,
                span,
                language,
                StagingStructureSemanticKind::Text {
                    text: text_span_value(*text_span, &self.text_buffers)?.to_owned(),
                },
            ),
            WireStagingM4Inline::InlineMath { speech, .. } => self.push_record(
                raw_node_id,
                Some(parent),
                None,
                span,
                language,
                StagingStructureSemanticKind::InlineMath {
                    alternative: speech.clone(),
                },
            ),
            WireStagingM4Inline::InlineVector { .. } => {
                if self.generation == StructureSemanticGeneration::V1 {
                    return Err(StagingStructureSemanticError::PrecomposedVectorStaging(
                        node,
                    ));
                }
                let metrics = self
                    .package
                    .precomposed_vector_metrics_for(node)
                    .ok_or(StagingStructureSemanticError::ReceiptMismatch)?;
                self.package
                    .verify_precomposed_vector_metrics(metrics)
                    .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?;
                if metrics.kind() != PrecomposedVectorKind::InlineVector {
                    return Err(StagingStructureSemanticError::ReceiptMismatch);
                }
                self.push_record(
                    raw_node_id,
                    Some(parent),
                    None,
                    span,
                    language,
                    StagingStructureSemanticKind::InlineVector {
                        alternative: metrics.alternative().alternative().to_owned(),
                        authored_actual_text: metrics
                            .alternative()
                            .authored_actual_text()
                            .map(str::to_owned),
                        metrics_fingerprint: metrics.fingerprint(),
                    },
                )
            }
            WireStagingM4Inline::MathVector { .. } => {
                if self.generation == StructureSemanticGeneration::V1 {
                    return Err(StagingStructureSemanticError::PrecomposedVectorStaging(
                        node,
                    ));
                }
                let metrics = self
                    .package
                    .precomposed_vector_metrics_for(node)
                    .ok_or(StagingStructureSemanticError::ReceiptMismatch)?;
                self.package
                    .verify_precomposed_vector_metrics(metrics)
                    .map_err(|_| StagingStructureSemanticError::ReceiptMismatch)?;
                if metrics.kind() != PrecomposedVectorKind::MathVector {
                    return Err(StagingStructureSemanticError::ReceiptMismatch);
                }
                self.push_record(
                    raw_node_id,
                    Some(parent),
                    None,
                    span,
                    language,
                    StagingStructureSemanticKind::MathVector {
                        alternative: metrics.alternative().alternative().to_owned(),
                        resolved_actual_text: metrics
                            .alternative()
                            .resolved_actual_text()
                            .ok_or(StagingStructureSemanticError::ReceiptMismatch)?
                            .to_owned(),
                        metrics_fingerprint: metrics.fingerprint(),
                    },
                )
            }
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. } => {
                let kind = if matches!(value, WireStagingM4Inline::Emphasis { .. }) {
                    StagingStructureSemanticKind::Emphasis
                } else {
                    StagingStructureSemanticKind::Strong
                };
                self.push_record(
                    raw_node_id,
                    Some(parent),
                    None,
                    span,
                    language.clone(),
                    kind,
                )?;
                for child in children {
                    self.inline(
                        child,
                        node,
                        paragraph_owner,
                        direct_paragraph_branch,
                        &language,
                        in_link,
                        footnote_container_valid,
                    )?;
                }
                Ok(())
            }
            WireStagingM4Inline::Link { children, .. } => {
                let accessible_name = inline_text(
                    children,
                    &self.text_buffers,
                    &self.footnote_markers,
                    self.generation,
                )?;
                self.push_record(
                    raw_node_id,
                    Some(parent),
                    None,
                    span,
                    language.clone(),
                    StagingStructureSemanticKind::Link { accessible_name },
                )?;
                for child in children {
                    self.inline(
                        child,
                        node,
                        paragraph_owner,
                        direct_paragraph_branch,
                        &language,
                        true,
                        footnote_container_valid,
                    )?;
                }
                Ok(())
            }
            WireStagingM4Inline::Anchor { .. } => self.push_record(
                raw_node_id,
                Some(parent),
                None,
                span,
                language,
                StagingStructureSemanticKind::Anchor,
            ),
            WireStagingM4Inline::Reference { target, format, .. } => self.push_record(
                raw_node_id,
                Some(parent),
                None,
                span,
                language,
                StagingStructureSemanticKind::Reference {
                    label: reference_label(target, *format),
                },
            ),
            WireStagingM4Inline::FootnoteReference { footnote_id, .. } => {
                let placement_valid = footnote_container_valid && !in_link;
                let marker = self
                    .footnote_markers
                    .get(footnote_id)
                    .cloned()
                    .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?;
                self.footnote_references
                    .entry(footnote_id.clone())
                    .or_default()
                    .push(FootnoteReferenceSite {
                        node_id: node,
                        paragraph_owner: Some(paragraph_owner),
                        direct_paragraph_branch: Some(direct_paragraph_branch),
                        placement_valid,
                    });
                self.push_record(
                    raw_node_id,
                    Some(parent),
                    None,
                    span,
                    language,
                    StagingStructureSemanticKind::FootnoteReference {
                        footnote_id: footnote_id.clone(),
                        marker,
                        placement_valid,
                    },
                )
            }
            WireStagingM4Inline::SoftBreak { .. } => self.push_record(
                raw_node_id,
                Some(parent),
                None,
                span,
                language,
                StagingStructureSemanticKind::SoftBreak,
            ),
            WireStagingM4Inline::HardBreak { .. } => self.push_record(
                raw_node_id,
                Some(parent),
                None,
                span,
                language,
                StagingStructureSemanticKind::HardBreak,
            ),
        }
    }

    fn footnotes(
        &mut self,
        document: &WireStagingM4Document,
    ) -> Result<(), StagingStructureSemanticError> {
        let document_id = NodeId::new(document.node_id);
        for footnote in &document.footnotes {
            let sites_before_definition = self
                .footnote_references
                .get(&footnote.footnote_id)
                .cloned()
                .unwrap_or_default();
            let last = sites_before_definition
                .iter()
                .filter(|site| site.placement_valid)
                .max_by_key(|site| site.node_id);
            let parent = last
                .and_then(|site| site.paragraph_owner)
                .unwrap_or(document_id);
            let insertion_after = last.and_then(|site| site.direct_paragraph_branch);
            let marker = self
                .footnote_markers
                .get(&footnote.footnote_id)
                .cloned()
                .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?;
            let language = self.language(footnote.node_id, document.language.as_str())?;
            let footnote_node = NodeId::new(footnote.node_id);
            let definition_record_index = self.records.len();
            self.push_record(
                footnote.node_id,
                Some(parent),
                insertion_after,
                Some(footnote.span),
                language.clone(),
                StagingStructureSemanticKind::FootnoteDefinition {
                    footnote_id: footnote.footnote_id.clone(),
                    marker: marker.clone(),
                    reference_node_ids: sites_before_definition
                        .iter()
                        .map(|site| site.node_id)
                        .collect(),
                    placement_valid: false,
                },
            )?;
            self.blocks(&footnote.blocks, footnote_node, &language, false)?;
            let sites = self
                .footnote_references
                .get(&footnote.footnote_id)
                .cloned()
                .unwrap_or_default();
            let placement_valid = !sites.is_empty()
                && sites
                    .iter()
                    .all(|site| site.placement_valid && site.direct_paragraph_branch.is_some());
            let record = self
                .records
                .get_mut(definition_record_index)
                .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?;
            record.kind = StagingStructureSemanticKind::FootnoteDefinition {
                footnote_id: footnote.footnote_id.clone(),
                marker,
                reference_node_ids: sites.iter().map(|site| site.node_id).collect(),
                placement_valid,
            };
        }
        let definitions = document
            .footnotes
            .iter()
            .map(|value| value.footnote_id.as_str())
            .collect::<BTreeSet<_>>();
        if self
            .footnote_references
            .keys()
            .any(|footnote_id| !definitions.contains(footnote_id.as_str()))
        {
            return Err(StagingStructureSemanticError::InvalidSemanticTree);
        }
        Ok(())
    }
}

fn raw_block_span(value: &WireStagingM4Block) -> WireStagingSourceSpan {
    match value {
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

fn lower_span(value: WireStagingSourceSpan) -> Result<SourceSpan, StagingStructureSemanticError> {
    SourceSpan::new(
        SourceId::new(value.source_id),
        Utf8ByteOffset::new(value.start_byte),
        Utf8ByteOffset::new(value.end_byte),
    )
    .ok_or(StagingStructureSemanticError::InvalidSemanticTree)
}

fn text_span_value<'a>(
    span: typaxis_document_package::WireStagingTextSpan,
    buffers: &BTreeMap<u32, &'a str>,
) -> Result<&'a str, StagingStructureSemanticError> {
    let value = buffers
        .get(&span.text_id)
        .ok_or(StagingStructureSemanticError::InvalidTextBinding)?;
    value
        .get(span.start_byte as usize..span.end_byte as usize)
        .ok_or(StagingStructureSemanticError::InvalidTextBinding)
}

fn inline_text(
    values: &[WireStagingM4Inline],
    buffers: &BTreeMap<u32, &str>,
    footnote_markers: &BTreeMap<String, String>,
    generation: StructureSemanticGeneration,
) -> Result<String, StagingStructureSemanticError> {
    let mut output = String::new();
    for value in values {
        match value {
            WireStagingM4Inline::Text { text_span, .. } => {
                output.push_str(text_span_value(*text_span, buffers)?);
            }
            WireStagingM4Inline::InlineMath { speech, .. } => output.push_str(speech),
            WireStagingM4Inline::InlineVector { node_id, alt, .. }
            | WireStagingM4Inline::MathVector { node_id, alt, .. } => match generation {
                StructureSemanticGeneration::V1 => {
                    return Err(StagingStructureSemanticError::PrecomposedVectorStaging(
                        NodeId::new(*node_id),
                    ));
                }
                StructureSemanticGeneration::V2 => output.push_str(alt),
            },
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } => {
                output.push_str(&inline_text(
                    children,
                    buffers,
                    footnote_markers,
                    generation,
                )?);
            }
            WireStagingM4Inline::Reference { target, format, .. } => {
                output.push_str(&reference_label(target, *format));
            }
            WireStagingM4Inline::FootnoteReference { footnote_id, .. } => {
                output.push_str(
                    footnote_markers
                        .get(footnote_id)
                        .ok_or(StagingStructureSemanticError::InvalidSemanticTree)?,
                );
            }
            WireStagingM4Inline::SoftBreak { .. } | WireStagingM4Inline::HardBreak { .. } => {
                output.push(' ');
            }
            WireStagingM4Inline::Anchor { .. } => {}
        }
    }
    Ok(output)
}

fn blocks_have_content(
    values: &[WireStagingM4Block],
    buffers: &BTreeMap<u32, &str>,
    footnote_markers: &BTreeMap<String, String>,
    generation: StructureSemanticGeneration,
) -> Result<bool, StagingStructureSemanticError> {
    for value in values {
        let has_content = match value {
            WireStagingM4Block::Paragraph { children, .. }
            | WireStagingM4Block::Heading { children, .. } => has_non_whitespace(&inline_text(
                children,
                buffers,
                footnote_markers,
                generation,
            )?),
            WireStagingM4Block::List { items, .. } => {
                let mut found = false;
                for item in items {
                    found |=
                        blocks_have_content(&item.blocks, buffers, footnote_markers, generation)?;
                }
                found
            }
            WireStagingM4Block::Table { head, body, .. } => {
                let mut found = false;
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    found |=
                        blocks_have_content(&cell.blocks, buffers, footnote_markers, generation)?;
                }
                found
            }
            WireStagingM4Block::Figure { alt, caption, .. } => {
                has_non_whitespace(alt)
                    || blocks_have_content(caption, buffers, footnote_markers, generation)?
            }
            WireStagingM4Block::DisplayMath { speech, .. } => has_non_whitespace(speech),
            WireStagingM4Block::VectorFigure { node_id, .. }
            | WireStagingM4Block::MathVectorBlock { node_id, .. } => match generation {
                StructureSemanticGeneration::V1 => {
                    return Err(StagingStructureSemanticError::PrecomposedVectorStaging(
                        NodeId::new(*node_id),
                    ));
                }
                StructureSemanticGeneration::V2 => true,
            },
            WireStagingM4Block::SemanticContainer { blocks, .. } => {
                blocks_have_content(blocks, buffers, footnote_markers, generation)?
            }
            WireStagingM4Block::PageBreak { .. } => false,
        };
        if has_content {
            return Ok(true);
        }
    }
    Ok(false)
}

fn reference_label(target: &str, format: WireStagingM4ReferenceFormat) -> String {
    match format {
        WireStagingM4ReferenceFormat::Text => target.to_owned(),
        WireStagingM4ReferenceFormat::Page => format!("page {target}"),
        WireStagingM4ReferenceFormat::Number => format!("number {target}"),
    }
}

fn has_non_whitespace(value: &str) -> bool {
    value.chars().any(|character| !character.is_whitespace())
}

fn row_origins(
    rows: &[WireStagingM4TableRow],
    column_count: u16,
) -> Result<Vec<Vec<u16>>, StagingStructureSemanticError> {
    if column_count == 0 {
        return Err(StagingStructureSemanticError::InvalidTableGrid);
    }
    let mut remaining = vec![0u16; usize::from(column_count)];
    let mut output = Vec::new();
    output
        .try_reserve_exact(rows.len())
        .map_err(|_| StagingStructureSemanticError::AllocationFailure)?;
    for row in rows {
        let mut origins = Vec::new();
        origins
            .try_reserve_exact(row.cells.len())
            .map_err(|_| StagingStructureSemanticError::AllocationFailure)?;
        let mut cursor = 0usize;
        for cell in &row.cells {
            while cursor < remaining.len() && remaining[cursor] != 0 {
                cursor += 1;
            }
            let width = usize::from(cell.colspan);
            let end = cursor
                .checked_add(width)
                .ok_or(StagingStructureSemanticError::InvalidTableGrid)?;
            if cell.colspan == 0
                || cell.rowspan == 0
                || end > remaining.len()
                || remaining[cursor..end].iter().any(|value| *value != 0)
            {
                return Err(StagingStructureSemanticError::InvalidTableGrid);
            }
            origins.push(
                u16::try_from(cursor)
                    .map_err(|_| StagingStructureSemanticError::InvalidTableGrid)?,
            );
            for slot in &mut remaining[cursor..end] {
                *slot = cell.rowspan;
            }
            cursor = end;
        }
        if remaining.contains(&0) {
            return Err(StagingStructureSemanticError::InvalidTableGrid);
        }
        for value in &mut remaining {
            *value = value.saturating_sub(1);
        }
        output.push(origins);
    }
    if remaining.iter().any(|value| *value != 0) {
        return Err(StagingStructureSemanticError::InvalidTableGrid);
    }
    Ok(output)
}

fn encode_semantics(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    records: &[StagingStructureSemanticRecord],
    limits_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_STRUCTURE_SEMANTIC_INPUT_ALGORITHM);
    output.push_str(",\"language_sha256\":");
    push_hash(&mut output, navigation.languages().fingerprint());
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, limits_sha256);
    output.push_str(",\"metadata_sha256\":");
    push_hash(&mut output, navigation.metadata().fingerprint());
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, navigation.outline().fingerprint());
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"records\":[");
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        encode_record(&mut output, record);
    }
    output.push_str("],\"semantic_sha256\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push('}');
    output
}

fn encode_semantics_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    records: &[StagingStructureSemanticRecord],
    limits_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"language_sha256\":");
    push_hash(&mut output, navigation.languages().fingerprint());
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, limits_sha256);
    output.push_str(",\"metadata_sha256\":");
    push_hash(&mut output, navigation.metadata().fingerprint());
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, navigation.outline().fingerprint());
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"records\":[");
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        encode_record_v2(&mut output, record);
    }
    output.push_str("],\"semantic_sha256\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"structure_registry_algorithm\":");
    // This is an internal projection of the adopted structure-registry `/2`
    // identity, not an independently versioned cross-crate algorithm.
    push_jcs_string(&mut output, "typaxis.structure-registry/2");
    output.push('}');
    output
}

fn encode_record(output: &mut String, record: &StagingStructureSemanticRecord) {
    output.push_str("{\"insertion_after_node_id\":");
    push_optional_node(output, record.insertion_after_node_id);
    output.push_str(",\"kind\":");
    push_jcs_string(output, record.kind.as_str());
    output.push_str(",\"language\":");
    push_jcs_string(output, &record.language);
    output.push_str(",\"node_id\":");
    output.push_str(&record.node_id.get().to_string());
    output.push_str(",\"outline_ids\":[");
    for (index, outline_id) in record.outline_ids.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&outline_id.to_string());
    }
    output.push_str("],\"parent_node_id\":");
    push_optional_node(output, record.parent_node_id);
    output.push_str(",\"properties\":");
    encode_kind_properties(output, &record.kind);
    output.push_str(",\"source_span\":");
    if let Some(span) = record.source_span {
        output.push_str("{\"end_byte\":");
        output.push_str(&span.end_byte().get().to_string());
        output.push_str(",\"source_id\":");
        output.push_str(&span.source_id().get().to_string());
        output.push_str(",\"start_byte\":");
        output.push_str(&span.start_byte().get().to_string());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push('}');
}

fn encode_record_v2(output: &mut String, record: &StagingStructureSemanticRecord) {
    output.push_str("{\"insertion_after_node_id\":");
    push_optional_node(output, record.insertion_after_node_id);
    output.push_str(",\"kind\":");
    push_jcs_string(output, record.kind.as_str());
    output.push_str(",\"language\":");
    push_jcs_string(output, &record.language);
    output.push_str(",\"language_binding\":");
    if let Some(binding) = record.language_binding_v2 {
        output.push_str("{\"parent_record_fingerprint\":");
        push_optional_hash(output, binding.parent_record_fingerprint);
        output.push_str(",\"record_fingerprint\":");
        push_hash(output, binding.record_fingerprint);
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"node_id\":");
    output.push_str(&record.node_id.get().to_string());
    output.push_str(",\"outline_ids\":[");
    for (index, outline_id) in record.outline_ids.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&outline_id.to_string());
    }
    output.push_str("],\"parent_node_id\":");
    push_optional_node(output, record.parent_node_id);
    output.push_str(",\"properties\":");
    encode_kind_properties(output, &record.kind);
    output.push_str(",\"source_span\":");
    if let Some(span) = record.source_span {
        push_source_span(output, span);
    } else {
        output.push_str("null");
    }
    output.push('}');
}

#[allow(clippy::too_many_lines)]
fn encode_kind_properties(output: &mut String, kind: &StagingStructureSemanticKind) {
    match kind {
        StagingStructureSemanticKind::Document
        | StagingStructureSemanticKind::PageBreak
        | StagingStructureSemanticKind::Emphasis
        | StagingStructureSemanticKind::Strong
        | StagingStructureSemanticKind::Anchor
        | StagingStructureSemanticKind::SoftBreak
        | StagingStructureSemanticKind::HardBreak => output.push_str("{}"),
        StagingStructureSemanticKind::SemanticContainer { semantic_kind } => {
            output.push_str("{\"semantic_kind\":");
            push_jcs_string(output, semantic_kind);
            output.push('}');
        }
        StagingStructureSemanticKind::Paragraph { has_real_content } => {
            output.push_str("{\"has_real_content\":");
            output.push_str(if *has_real_content { "true" } else { "false" });
            output.push('}');
        }
        StagingStructureSemanticKind::Heading {
            level,
            has_real_content,
        } => {
            output.push_str("{\"has_real_content\":");
            output.push_str(if *has_real_content { "true" } else { "false" });
            output.push_str(",\"level\":");
            output.push_str(&level.to_string());
            output.push('}');
        }
        StagingStructureSemanticKind::List { ordered } => {
            output.push_str("{\"ordered\":");
            output.push_str(if *ordered { "true" } else { "false" });
            output.push('}');
        }
        StagingStructureSemanticKind::ListItem { marker } => {
            output.push_str("{\"marker\":");
            push_jcs_string(output, marker);
            output.push('}');
        }
        StagingStructureSemanticKind::Table {
            head_rows,
            body_rows,
        } => {
            output.push_str("{\"body_rows\":");
            output.push_str(&body_rows.to_string());
            output.push_str(",\"head_rows\":");
            output.push_str(&head_rows.to_string());
            output.push('}');
        }
        StagingStructureSemanticKind::TableRow {
            section,
            row_ordinal,
        } => {
            output.push_str("{\"row_ordinal\":");
            output.push_str(&row_ordinal.to_string());
            output.push_str(",\"section\":");
            push_jcs_string(output, section.as_str());
            output.push('}');
        }
        StagingStructureSemanticKind::TableCell {
            section,
            row_ordinal,
            column_ordinal,
            colspan,
            rowspan,
            header_node_ids,
            has_real_content,
        } => {
            output.push_str("{\"colspan\":");
            output.push_str(&colspan.to_string());
            output.push_str(",\"column_ordinal\":");
            output.push_str(&column_ordinal.to_string());
            output.push_str(",\"has_real_content\":");
            output.push_str(if *has_real_content { "true" } else { "false" });
            output.push_str(",\"header_node_ids\":[");
            for (index, node_id) in header_node_ids.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(&node_id.get().to_string());
            }
            output.push_str("],\"row_ordinal\":");
            output.push_str(&row_ordinal.to_string());
            output.push_str(",\"rowspan\":");
            output.push_str(&rowspan.to_string());
            output.push_str(",\"section\":");
            push_jcs_string(output, section.as_str());
            output.push('}');
        }
        StagingStructureSemanticKind::Figure {
            alternative,
            has_caption,
        } => {
            output.push_str("{\"alternative\":");
            push_jcs_string(output, alternative);
            output.push_str(",\"has_caption\":");
            output.push_str(if *has_caption { "true" } else { "false" });
            output.push('}');
        }
        StagingStructureSemanticKind::DisplayMath { alternative }
        | StagingStructureSemanticKind::InlineMath { alternative } => {
            output.push_str("{\"alternative\":");
            push_jcs_string(output, alternative);
            output.push('}');
        }
        StagingStructureSemanticKind::InlineVector {
            alternative,
            authored_actual_text,
            metrics_fingerprint,
        } => {
            output.push_str("{\"alternative\":");
            push_jcs_string(output, alternative);
            output.push_str(",\"authored_actual_text\":");
            push_optional_string(output, authored_actual_text.as_deref());
            output.push_str(",\"metrics_fingerprint\":");
            push_hash(output, *metrics_fingerprint);
            output.push('}');
        }
        StagingStructureSemanticKind::MathVector {
            alternative,
            resolved_actual_text,
            metrics_fingerprint,
        } => {
            encode_math_vector_properties(
                output,
                alternative,
                resolved_actual_text,
                *metrics_fingerprint,
                None,
            );
        }
        StagingStructureSemanticKind::VectorFigure {
            alternative,
            has_caption,
            metrics_fingerprint,
        } => {
            output.push_str("{\"alternative\":");
            push_jcs_string(output, alternative);
            output.push_str(",\"has_caption\":");
            output.push_str(if *has_caption { "true" } else { "false" });
            output.push_str(",\"metrics_fingerprint\":");
            push_hash(output, *metrics_fingerprint);
            output.push('}');
        }
        StagingStructureSemanticKind::MathVectorBlock {
            alternative,
            resolved_actual_text,
            metrics_fingerprint,
            equation_number_node_id,
        } => {
            encode_math_vector_properties(
                output,
                alternative,
                resolved_actual_text,
                *metrics_fingerprint,
                Some(*equation_number_node_id),
            );
        }
        StagingStructureSemanticKind::EquationNumber { binding } => {
            output.push_str("{\"exact_text\":");
            push_jcs_string(output, binding.exact_text());
            output.push_str(",\"exact_text_sha256\":");
            push_hash(output, binding.exact_text_sha256());
            output.push_str(",\"parent_owner\":");
            output.push_str(&binding.parent_owner().get().to_string());
            output.push_str(",\"text_buffer_sha256\":");
            push_hash(output, binding.text_buffer_sha256());
            output.push_str(",\"text_span\":");
            push_text_span(output, binding.text_span());
            output.push('}');
        }
        StagingStructureSemanticKind::FootnoteDefinition {
            footnote_id,
            marker,
            reference_node_ids,
            placement_valid,
        } => {
            output.push_str("{\"footnote_id\":");
            push_jcs_string(output, footnote_id);
            output.push_str(",\"marker\":");
            push_jcs_string(output, marker);
            output.push_str(",\"placement_valid\":");
            output.push_str(if *placement_valid { "true" } else { "false" });
            output.push_str(",\"reference_node_ids\":[");
            for (index, node_id) in reference_node_ids.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(&node_id.get().to_string());
            }
            output.push_str("]}");
        }
        StagingStructureSemanticKind::Text { text } => {
            output.push_str("{\"text\":");
            push_jcs_string(output, text);
            output.push('}');
        }
        StagingStructureSemanticKind::Link { accessible_name } => {
            output.push_str("{\"accessible_name\":");
            push_jcs_string(output, accessible_name);
            output.push('}');
        }
        StagingStructureSemanticKind::Reference { label } => {
            output.push_str("{\"label\":");
            push_jcs_string(output, label);
            output.push('}');
        }
        StagingStructureSemanticKind::FootnoteReference {
            footnote_id,
            marker,
            placement_valid,
        } => {
            output.push_str("{\"footnote_id\":");
            push_jcs_string(output, footnote_id);
            output.push_str(",\"marker\":");
            push_jcs_string(output, marker);
            output.push_str(",\"placement_valid\":");
            output.push_str(if *placement_valid { "true" } else { "false" });
            output.push('}');
        }
    }
}

fn encode_math_vector_properties(
    output: &mut String,
    alternative: &str,
    resolved_actual_text: &str,
    metrics_fingerprint: [u8; 32],
    equation_number_node_id: Option<Option<NodeId>>,
) {
    output.push_str("{\"alternative\":");
    push_jcs_string(output, alternative);
    if let Some(node_id) = equation_number_node_id {
        output.push_str(",\"equation_number_node_id\":");
        push_optional_node(output, node_id);
    }
    output.push_str(",\"metrics_fingerprint\":");
    push_hash(output, metrics_fingerprint);
    output.push_str(",\"resolved_actual_text\":");
    push_jcs_string(output, resolved_actual_text);
    output.push('}');
}

fn encode_profile_view(value: &StagingAccessibilityProfileView) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_ACCESSIBILITY_PROFILE_VIEW_ALGORITHM);
    output.push_str(",\"language_sha256\":");
    push_hash(&mut output, value.language_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"metadata_sha256\":");
    push_hash(&mut output, value.metadata_sha256);
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, value.outline_sha256);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, value.package_sha256);
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, value.semantic_sha256);
    output.push_str(",\"structure_semantics_sha256\":");
    push_hash(&mut output, value.structure_semantics_sha256);
    output.push('}');
    output
}

fn encode_profile_view_v2(value: &StagingAccessibilityProfileViewV2) -> String {
    let mut output = String::from("{\"language_sha256\":");
    push_hash(&mut output, value.language_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"metadata_sha256\":");
    push_hash(&mut output, value.metadata_sha256);
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, value.outline_sha256);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, value.package_sha256);
    output.push_str(",\"production_accessibility_preflight_algorithm\":");
    // The view is a component projection of the adopted accessibility
    // preflight `/2` identity and does not introduce a sibling identity.
    push_jcs_string(&mut output, "typaxis.production-accessibility-preflight/2");
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, value.semantic_sha256);
    output.push_str(",\"structure_semantics_sha256\":");
    push_hash(&mut output, value.structure_semantics_sha256);
    output.push('}');
    output
}

fn encode_authorization(
    view: &StagingAccessibilityProfileView,
    profile_receipt_fingerprint: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_ACCESSIBILITY_AUTHORIZATION_ALGORITHM);
    output.push_str(",\"profile_receipt_sha256\":");
    push_hash(&mut output, profile_receipt_fingerprint);
    output.push_str(",\"profile_view_sha256\":");
    push_hash(&mut output, view.fingerprint());
    output.push('}');
    output
}

fn encode_authorization_v2(
    view: &StagingAccessibilityProfileViewV2,
    profile_receipt_fingerprint: [u8; 32],
    book_navigation_profile_fingerprint: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(
        &mut output,
        STAGING_ACCESSIBILITY_AUTHORIZATION_ALGORITHM_V2,
    );
    output.push_str(",\"book_navigation_profile_sha256\":");
    push_hash(&mut output, book_navigation_profile_fingerprint);
    output.push_str(",\"profile_receipt_sha256\":");
    push_hash(&mut output, profile_receipt_fingerprint);
    output.push_str(",\"profile_view_sha256\":");
    push_hash(&mut output, view.fingerprint());
    output.push('}');
    output
}

fn push_optional_node(output: &mut String, value: Option<NodeId>) {
    if let Some(value) = value {
        output.push_str(&value.get().to_string());
    } else {
        output.push_str("null");
    }
}

fn push_optional_string(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_optional_hash(output: &mut String, value: Option<[u8; 32]>) {
    if let Some(value) = value {
        push_hash(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_source_span(output: &mut String, value: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push('}');
}

fn push_text_span(output: &mut String, value: TextSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&value.text_id().get().to_string());
    output.push('}');
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

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_document_package::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireStagingM4Block, WireStagingM4Inline,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/accessibility/job/document-package.json"
    ));

    #[test]
    fn tagged_structure_semantics_are_dense_complete_and_deterministic() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = crate::StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = crate::validate_staging_book_navigation(&package, &limits).unwrap();
        let first = validate_staging_structure_semantics(&package, &navigation).unwrap();
        let second = validate_staging_structure_semantics(&package, &navigation).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.record(NodeId::new(0)).unwrap().parent_node_id(), None);
        assert!(first
            .records()
            .iter()
            .enumerate()
            .all(|(index, record)| { usize::try_from(record.node_id().get()) == Ok(index) }));
        for required in [
            "document",
            "paragraph",
            "list",
            "table",
            "figure",
            "display_math",
            "inline_math",
            "link",
            "footnote_definition",
            "semantic_container",
        ] {
            assert!(first
                .records()
                .iter()
                .any(|record| record.kind().as_str() == required));
        }
        assert!(first.records().iter().any(|record| matches!(
            record.kind(),
            StagingStructureSemanticKind::ListItem { marker } if marker == "1."
        )));
        let footnote_markers = first
            .records()
            .iter()
            .filter_map(|record| match record.kind() {
                StagingStructureSemanticKind::FootnoteDefinition { marker, .. }
                | StagingStructureSemanticKind::FootnoteReference { marker, .. } => {
                    Some(marker.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!footnote_markers.is_empty());
        assert!(footnote_markers.iter().all(|marker| *marker == "1"));
        first.verify(&package, &navigation).unwrap();
    }

    #[test]
    fn footnote_definition_references_are_not_body_placements() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        let footnote_id = document.footnotes[0].footnote_id.clone();
        let footnote_node_id = document.footnotes[0].node_id;
        let WireStagingM4Block::Paragraph { children, .. } = &mut document.footnotes[0].blocks[0]
        else {
            panic!("fixture footnote must begin with a paragraph");
        };
        let node_id = children[0].node_id();
        let span = children[0].span();
        children[0] = WireStagingM4Inline::FootnoteReference {
            node_id,
            span,
            footnote_id,
            language: None,
        };
        wire.replace_typed_regions(document, wire.resources().clone());
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
        let navigation = crate::validate_staging_book_navigation(&package, &limits).unwrap();
        let semantics = validate_staging_structure_semantics(&package, &navigation).unwrap();
        assert!(matches!(
            semantics.record(NodeId::new(node_id)).unwrap().kind(),
            StagingStructureSemanticKind::FootnoteReference {
                placement_valid: false,
                ..
            }
        ));
        assert!(matches!(
            semantics.record(NodeId::new(footnote_node_id)).unwrap().kind(),
            StagingStructureSemanticKind::FootnoteDefinition {
                placement_valid: false,
                reference_node_ids,
                ..
            } if reference_node_ids.contains(&NodeId::new(node_id))
        ));
    }
}
