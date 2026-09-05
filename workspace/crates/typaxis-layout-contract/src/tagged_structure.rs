use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{
    push_jcs_string, sha256, M4EffectiveResourceLimits, NodeId, SourceSpan, TextSpan,
    ValidatedResourceLimits,
};
use typaxis_syntax::{
    PrecomposedVectorKind, StagingAccessibilityProfileAuthorization,
    StagingAccessibilityProfileAuthorizationV2, StagingStructureLanguageBindingV2,
    StagingStructureSemanticKind, StagingStructureSemanticRecord, StagingStructureTableSection,
    ValidatedStagingBookNavigation, ValidatedStagingBookNavigationV2,
    ValidatedStagingSemanticPackage, ValidatedStagingStructureSemantics,
    ValidatedStagingStructureSemanticsV2,
};

pub const STRUCTURE_REGISTRY_ALGORITHM: &str = "typaxis.structure-registry/1";
pub const SELECTED_STRUCTURE_BINDING_ALGORITHM: &str = "typaxis.selected-structure-binding/1";
pub const STRUCTURE_ROLE_VOCABULARY_ALGORITHM_V2: &str = "typaxis.structure-role-vocabulary/2";
pub const STRUCTURE_REGISTRY_ALGORITHM_V2: &str = "typaxis.structure-registry/2";
pub const SELECTED_STRUCTURE_BINDING_ALGORITHM_V2: &str = "typaxis.selected-structure-binding/2";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructureNodeId(u32);

impl StructureNodeId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GeneratedStructureSlot {
    ListLabel,
    ListBody,
    TableHead,
    TableBody,
    FigureCaption,
    FootnoteLabel,
}

impl GeneratedStructureSlot {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListLabel => "list_label",
            Self::ListBody => "list_body",
            Self::TableHead => "table_head",
            Self::TableBody => "table_body",
            Self::FigureCaption => "figure_caption",
            Self::FootnoteLabel => "footnote_label",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratedStructureKey {
    owner_node_id: NodeId,
    slot: GeneratedStructureSlot,
    ordinal: u32,
}

impl GeneratedStructureKey {
    pub const fn new(owner_node_id: NodeId, slot: GeneratedStructureSlot) -> Self {
        Self {
            owner_node_id,
            slot,
            ordinal: 0,
        }
    }
    pub const fn owner_node_id(self) -> NodeId {
        self.owner_node_id
    }
    pub const fn slot(self) -> GeneratedStructureSlot {
        self.slot
    }
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StructureOwner {
    Source(NodeId),
    Generated(GeneratedStructureKey),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StructureRole {
    Document,
    Result,
    Proof,
    Exercise,
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    List,
    ListItem,
    Label,
    ListBody,
    Table,
    TableHead,
    TableBody,
    TableRow,
    TableHeader,
    TableData,
    Figure,
    Caption,
    Formula,
    Note,
    Span,
    Emphasis,
    Strong,
    Link,
    Reference,
}

impl StructureRole {
    pub const fn pdf_name(self) -> &'static str {
        match self {
            Self::Document => "Document",
            Self::Result => "Result",
            Self::Proof => "Proof",
            Self::Exercise => "Exercise",
            Self::Paragraph => "P",
            Self::Heading1 => "H1",
            Self::Heading2 => "H2",
            Self::Heading3 => "H3",
            Self::Heading4 => "H4",
            Self::Heading5 => "H5",
            Self::Heading6 => "H6",
            Self::List => "L",
            Self::ListItem => "LI",
            Self::Label => "Lbl",
            Self::ListBody => "LBody",
            Self::Table => "Table",
            Self::TableHead => "THead",
            Self::TableBody => "TBody",
            Self::TableRow => "TR",
            Self::TableHeader => "TH",
            Self::TableData => "TD",
            Self::Figure => "Figure",
            Self::Caption => "Caption",
            Self::Formula => "Formula",
            Self::Note => "Note",
            Self::Span => "Span",
            Self::Emphasis => "Em",
            Self::Strong => "Strong",
            Self::Link => "Link",
            Self::Reference => "Reference",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructureRoleVocabularyV2;

impl StructureRoleVocabularyV2 {
    pub const fn roles(self) -> [StructureRole; 30] {
        [
            StructureRole::Caption,
            StructureRole::Document,
            StructureRole::Emphasis,
            StructureRole::Exercise,
            StructureRole::Figure,
            StructureRole::Formula,
            StructureRole::Heading1,
            StructureRole::Heading2,
            StructureRole::Heading3,
            StructureRole::Heading4,
            StructureRole::Heading5,
            StructureRole::Heading6,
            StructureRole::List,
            StructureRole::ListBody,
            StructureRole::ListItem,
            StructureRole::Label,
            StructureRole::Link,
            StructureRole::Note,
            StructureRole::Paragraph,
            StructureRole::Proof,
            StructureRole::Reference,
            StructureRole::Result,
            StructureRole::Span,
            StructureRole::Strong,
            StructureRole::TableBody,
            StructureRole::TableData,
            StructureRole::TableHeader,
            StructureRole::TableHead,
            StructureRole::TableRow,
            StructureRole::Table,
        ]
    }

    pub fn canonical_jcs(self) -> String {
        encode_structure_role_vocabulary_v2()
    }

    pub fn fingerprint(self) -> [u8; 32] {
        sha256(self.canonical_jcs().as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StructureListNumbering {
    Decimal,
    Disc,
}

impl StructureListNumbering {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::Disc => "disc",
        }
    }

    pub const fn pdf_name(self) -> &'static str {
        match self {
            Self::Decimal => "Decimal",
            Self::Disc => "Disc",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureTableAttributes {
    section: StagingStructureTableSection,
    row_ordinal: u32,
    column_ordinal: u32,
    colspan: u16,
    rowspan: u16,
    header_ids: Vec<String>,
}

impl StructureTableAttributes {
    pub const fn section(&self) -> StagingStructureTableSection {
        self.section
    }
    pub const fn row_ordinal(&self) -> u32 {
        self.row_ordinal
    }
    pub const fn column_ordinal(&self) -> u32 {
        self.column_ordinal
    }
    pub const fn colspan(&self) -> u16 {
        self.colspan
    }
    pub const fn rowspan(&self) -> u16 {
        self.rowspan
    }
    pub fn header_ids(&self) -> &[String] {
        &self.header_ids
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructureVectorBindingV2 {
    kind: PrecomposedVectorKind,
    metrics_fingerprint: [u8; 32],
}

impl StructureVectorBindingV2 {
    pub const fn kind(self) -> PrecomposedVectorKind {
        self.kind
    }

    pub const fn metrics_fingerprint(self) -> [u8; 32] {
        self.metrics_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureEquationNumberBindingV2 {
    parent_owner: NodeId,
    text_span: TextSpan,
    text_buffer_sha256: [u8; 32],
    exact_text: String,
    exact_text_sha256: [u8; 32],
}

impl StructureEquationNumberBindingV2 {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureNodeRecord {
    structure_node_id: StructureNodeId,
    owner: StructureOwner,
    source_span: Option<SourceSpan>,
    role: StructureRole,
    parent: Option<StructureNodeId>,
    children: Vec<StructureNodeId>,
    language: String,
    language_binding_v2: Option<StagingStructureLanguageBindingV2>,
    vector_binding_v2: Option<StructureVectorBindingV2>,
    equation_number_binding_v2: Option<StructureEquationNumberBindingV2>,
    list_numbering: Option<StructureListNumbering>,
    alternative: Option<String>,
    accessible_name: Option<String>,
    structure_id: Option<String>,
    table_attributes: Option<StructureTableAttributes>,
    outline_ids: Vec<u32>,
    related_nodes: Vec<StructureNodeId>,
    paint_required: bool,
    actual_text: Option<String>,
    marker: Option<String>,
}

impl StructureNodeRecord {
    pub const fn structure_node_id(&self) -> StructureNodeId {
        self.structure_node_id
    }
    pub const fn owner(&self) -> StructureOwner {
        self.owner
    }
    pub const fn source_span(&self) -> Option<SourceSpan> {
        self.source_span
    }
    pub const fn role(&self) -> StructureRole {
        self.role
    }
    pub const fn parent(&self) -> Option<StructureNodeId> {
        self.parent
    }
    pub fn children(&self) -> &[StructureNodeId] {
        &self.children
    }
    pub fn language(&self) -> &str {
        &self.language
    }
    pub const fn language_binding_v2(&self) -> Option<StagingStructureLanguageBindingV2> {
        self.language_binding_v2
    }
    pub const fn vector_binding_v2(&self) -> Option<StructureVectorBindingV2> {
        self.vector_binding_v2
    }
    pub const fn equation_number_binding_v2(&self) -> Option<&StructureEquationNumberBindingV2> {
        self.equation_number_binding_v2.as_ref()
    }
    pub const fn list_numbering(&self) -> Option<StructureListNumbering> {
        self.list_numbering
    }
    pub fn alternative(&self) -> Option<&str> {
        self.alternative.as_deref()
    }
    pub fn accessible_name(&self) -> Option<&str> {
        self.accessible_name.as_deref()
    }
    pub fn structure_id(&self) -> Option<&str> {
        self.structure_id.as_deref()
    }
    pub const fn table_attributes(&self) -> Option<&StructureTableAttributes> {
        self.table_attributes.as_ref()
    }
    pub fn outline_ids(&self) -> &[u32] {
        &self.outline_ids
    }
    pub fn related_nodes(&self) -> &[StructureNodeId] {
        &self.related_nodes
    }
    pub const fn paint_required(&self) -> bool {
        self.paint_required
    }
    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }
    pub fn marker(&self) -> Option<&str> {
        self.marker.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureRegistryReceipt {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    semantics_sha256: [u8; 32],
    authorization_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    generated_node_count: u32,
    maximum_depth: u32,
    nodes: Vec<StructureNodeRecord>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StructureRegistryReceipt {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn semantics_sha256(&self) -> [u8; 32] {
        self.semantics_sha256
    }
    pub const fn authorization_sha256(&self) -> [u8; 32] {
        self.authorization_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn generated_node_count(&self) -> u32 {
        self.generated_node_count
    }
    pub const fn maximum_depth(&self) -> u32 {
        self.maximum_depth
    }
    pub fn nodes(&self) -> &[StructureNodeRecord] {
        &self.nodes
    }
    pub fn node(&self, id: StructureNodeId) -> Option<&StructureNodeRecord> {
        self.nodes.get(id.get() as usize)
    }
    pub fn source_node(&self, source: NodeId) -> Option<&StructureNodeRecord> {
        self.nodes
            .iter()
            .find(|record| record.owner == StructureOwner::Source(source))
    }
    pub fn generated_node(&self, key: GeneratedStructureKey) -> Option<&StructureNodeRecord> {
        self.nodes
            .iter()
            .find(|record| record.owner == StructureOwner::Generated(key))
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
        semantics: &ValidatedStagingStructureSemantics,
        authorization: &StagingAccessibilityProfileAuthorization,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), StructureRegistryError> {
        let observed =
            build_structure_registry(package, navigation, semantics, authorization, limits)?;
        if self != &observed {
            return Err(StructureRegistryError::ReceiptMismatch);
        }
        Ok(())
    }

    fn verify_sealed(
        &self,
        authorization: &StagingAccessibilityProfileAuthorization,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), StructureRegistryError> {
        let canonical = encode_registry(self);
        if self.authorization_sha256 != authorization.fingerprint()
            || self.limits_sha256 != authorization.view().limits_sha256()
            || self.limits_sha256 != limits_fingerprint(limits)
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
            || self
                .nodes
                .iter()
                .enumerate()
                .any(|(index, node)| usize::try_from(node.structure_node_id.get()) != Ok(index))
        {
            return Err(StructureRegistryError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureRegistryReceiptV2 {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    semantics_sha256: [u8; 32],
    authorization_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    role_vocabulary_sha256: [u8; 32],
    generated_node_count: u32,
    maximum_depth: u32,
    nodes: Vec<StructureNodeRecord>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StructureRegistryReceiptV2 {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn semantics_sha256(&self) -> [u8; 32] {
        self.semantics_sha256
    }
    pub const fn authorization_sha256(&self) -> [u8; 32] {
        self.authorization_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn role_vocabulary_sha256(&self) -> [u8; 32] {
        self.role_vocabulary_sha256
    }
    pub const fn generated_node_count(&self) -> u32 {
        self.generated_node_count
    }
    pub const fn maximum_depth(&self) -> u32 {
        self.maximum_depth
    }
    pub fn nodes(&self) -> &[StructureNodeRecord] {
        &self.nodes
    }
    pub fn node(&self, id: StructureNodeId) -> Option<&StructureNodeRecord> {
        self.nodes.get(id.get() as usize)
    }
    pub fn source_node(&self, source: NodeId) -> Option<&StructureNodeRecord> {
        self.nodes
            .iter()
            .find(|record| record.owner == StructureOwner::Source(source))
    }
    pub fn generated_node(&self, key: GeneratedStructureKey) -> Option<&StructureNodeRecord> {
        self.nodes
            .iter()
            .find(|record| record.owner == StructureOwner::Generated(key))
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
        semantics: &ValidatedStagingStructureSemanticsV2,
        authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StructureRegistryError> {
        let observed =
            build_structure_registry_v2(package, navigation, semantics, authorization, limits)?;
        if self != &observed {
            return Err(StructureRegistryError::ReceiptMismatch);
        }
        Ok(())
    }

    fn verify_sealed(
        &self,
        authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StructureRegistryError> {
        let vocabulary = encode_structure_role_vocabulary_v2();
        let canonical = encode_registry_v2(self);
        if self.authorization_sha256 != authorization.fingerprint()
            || self.limits_sha256 != authorization.view().limits_sha256()
            || self.limits_sha256 != limits.fingerprint()
            || self.role_vocabulary_sha256 != sha256(vocabulary.as_bytes())
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
            || self
                .nodes
                .iter()
                .enumerate()
                .any(|(index, node)| usize::try_from(node.structure_node_id.get()) != Ok(index))
            || self
                .nodes
                .iter()
                .any(|node| node.language_binding_v2.is_none())
        {
            return Err(StructureRegistryError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructureRegistryError {
    AuthorizationMismatch,
    UnknownSemantic,
    InvalidParent,
    InvalidGeneratedNode,
    InvalidTableHeader,
    AstNodeLimit,
    AstDepthLimit,
    TextLimit,
    TextAggregateLimit,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StructureRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorizationMismatch => {
                formatter.write_str("I9190: structure authorization mismatch")
            }
            Self::UnknownSemantic => formatter.write_str("I9190: unknown structure semantic"),
            Self::InvalidParent => formatter.write_str("I9190: structure parent/order mismatch"),
            Self::InvalidGeneratedNode => {
                formatter.write_str("I9190: generated structure node mismatch")
            }
            Self::InvalidTableHeader => {
                formatter.write_str("I9190: table header structure mismatch")
            }
            Self::AstNodeLimit => formatter.write_str("P1120: structure node limit exceeded"),
            Self::AstDepthLimit => formatter.write_str("P1121: structure depth limit exceeded"),
            Self::TextLimit => formatter.write_str("T2100: structure string limit exceeded"),
            Self::TextAggregateLimit => {
                formatter.write_str("T2101: derived structure text limit exceeded")
            }
            Self::AllocationFailure => formatter.write_str("P1120: structure allocation failed"),
            Self::ReceiptMismatch => formatter.write_str("I9190: structure receipt mismatch"),
        }
    }
}

impl std::error::Error for StructureRegistryError {}

#[derive(Clone, Copy)]
enum StructureSemanticsRef<'a> {
    V1(&'a ValidatedStagingStructureSemantics),
    V2(&'a ValidatedStagingStructureSemanticsV2),
}

impl<'a> StructureSemanticsRef<'a> {
    fn records(self) -> &'a [StagingStructureSemanticRecord] {
        match self {
            Self::V1(value) => value.records(),
            Self::V2(value) => value.records(),
        }
    }

    fn record(self, node_id: NodeId) -> Option<&'a StagingStructureSemanticRecord> {
        match self {
            Self::V1(value) => value.record(node_id),
            Self::V2(value) => value.record(node_id),
        }
    }

    const fn is_v2(self) -> bool {
        matches!(self, Self::V2(_))
    }
}

struct RegistryBuilder<'a> {
    semantics: StructureSemanticsRef<'a>,
    limits: &'a ValidatedResourceLimits,
    nodes: Vec<StructureNodeRecord>,
    source_to_structure: BTreeMap<NodeId, StructureNodeId>,
    generated: BTreeSet<GeneratedStructureKey>,
    maximum_depth: u32,
    derived_text_bytes: u64,
}

pub fn build_structure_registry(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    semantics: &ValidatedStagingStructureSemantics,
    authorization: &StagingAccessibilityProfileAuthorization,
    limits: &ValidatedResourceLimits,
) -> Result<StructureRegistryReceipt, StructureRegistryError> {
    if package.limits() != limits
        || authorization
            .authorizes(package, navigation, semantics)
            .is_err()
        || authorization.view().limits_sha256() != limits_fingerprint(limits)
    {
        return Err(StructureRegistryError::AuthorizationMismatch);
    }
    let mut builder = RegistryBuilder {
        semantics: StructureSemanticsRef::V1(semantics),
        limits,
        nodes: Vec::new(),
        source_to_structure: BTreeMap::new(),
        generated: BTreeSet::new(),
        maximum_depth: 0,
        derived_text_bytes: 0,
    };
    let root = builder.visit_source(NodeId::new(0), None, 1)?;
    builder.close_footnote_relations()?;
    if root != Some(StructureNodeId::new(0))
        || builder.source_to_structure.len()
            != semantics
                .records()
                .iter()
                .filter(|record| record.kind().creates_structure_element())
                .count()
    {
        return Err(StructureRegistryError::InvalidParent);
    }
    let generated_node_count =
        u32::try_from(builder.generated.len()).map_err(|_| StructureRegistryError::AstNodeLimit)?;
    let total_ast = u64::try_from(semantics.records().len())
        .ok()
        .and_then(|source| source.checked_add(u64::from(generated_node_count)))
        .ok_or(StructureRegistryError::AstNodeLimit)?;
    if total_ast > limits.get().max_ast_nodes {
        return Err(StructureRegistryError::AstNodeLimit);
    }
    let mut receipt = StructureRegistryReceipt {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_sha256: package.semantic_fingerprint(),
        semantics_sha256: semantics.fingerprint(),
        authorization_sha256: authorization.fingerprint(),
        limits_sha256: limits_fingerprint(limits),
        generated_node_count,
        maximum_depth: builder.maximum_depth,
        nodes: builder.nodes,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_registry(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    receipt.verify_sealed(authorization, limits)?;
    Ok(receipt)
}

pub fn build_structure_registry_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
) -> Result<StructureRegistryReceiptV2, StructureRegistryError> {
    if package.limits() != limits.base()
        || authorization
            .authorizes(package, navigation, semantics, limits)
            .is_err()
        || authorization.view().limits_sha256() != limits.fingerprint()
    {
        return Err(StructureRegistryError::AuthorizationMismatch);
    }
    let mut builder = RegistryBuilder {
        semantics: StructureSemanticsRef::V2(semantics),
        limits: limits.base(),
        nodes: Vec::new(),
        source_to_structure: BTreeMap::new(),
        generated: BTreeSet::new(),
        maximum_depth: 0,
        derived_text_bytes: 0,
    };
    let root = builder.visit_source(NodeId::new(0), None, 1)?;
    builder.close_footnote_relations()?;
    if root != Some(StructureNodeId::new(0))
        || builder.source_to_structure.len()
            != semantics
                .records()
                .iter()
                .filter(|record| record.kind().creates_structure_element())
                .count()
    {
        return Err(StructureRegistryError::InvalidParent);
    }
    validate_v2_registry_nodes(semantics, &builder.nodes, &builder.source_to_structure)?;
    let generated_node_count =
        u32::try_from(builder.generated.len()).map_err(|_| StructureRegistryError::AstNodeLimit)?;
    let total_ast = u64::try_from(semantics.records().len())
        .ok()
        .and_then(|source| source.checked_add(u64::from(generated_node_count)))
        .ok_or(StructureRegistryError::AstNodeLimit)?;
    if total_ast > limits.base().get().max_ast_nodes {
        return Err(StructureRegistryError::AstNodeLimit);
    }
    let role_vocabulary = encode_structure_role_vocabulary_v2();
    let mut receipt = StructureRegistryReceiptV2 {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_sha256: package.semantic_fingerprint(),
        semantics_sha256: semantics.fingerprint(),
        authorization_sha256: authorization.fingerprint(),
        limits_sha256: limits.fingerprint(),
        role_vocabulary_sha256: sha256(role_vocabulary.as_bytes()),
        generated_node_count,
        maximum_depth: builder.maximum_depth,
        nodes: builder.nodes,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_registry_v2(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    receipt.verify_sealed(authorization, limits)?;
    Ok(receipt)
}

fn validate_v2_registry_nodes(
    semantics: &ValidatedStagingStructureSemanticsV2,
    nodes: &[StructureNodeRecord],
    source_to_structure: &BTreeMap<NodeId, StructureNodeId>,
) -> Result<(), StructureRegistryError> {
    for semantic in semantics.records() {
        if !semantic.kind().creates_structure_element() {
            continue;
        }
        let id = source_to_structure
            .get(&semantic.node_id())
            .copied()
            .ok_or(StructureRegistryError::InvalidParent)?;
        let node = nodes
            .get(id.get() as usize)
            .ok_or(StructureRegistryError::InvalidParent)?;
        if node.language_binding_v2 != semantic.language_binding_v2() {
            return Err(StructureRegistryError::InvalidParent);
        }
        match semantic.kind() {
            StagingStructureSemanticKind::MathVectorBlock {
                equation_number_node_id,
                ..
            } => {
                if node.role != StructureRole::Formula
                    || node.vector_binding_v2.map(StructureVectorBindingV2::kind)
                        != Some(PrecomposedVectorKind::MathVectorBlock)
                {
                    return Err(StructureRegistryError::UnknownSemantic);
                }
                match equation_number_node_id {
                    Some(source_child) => {
                        let child_id = source_to_structure
                            .get(source_child)
                            .copied()
                            .ok_or(StructureRegistryError::InvalidParent)?;
                        let child = nodes
                            .get(child_id.get() as usize)
                            .ok_or(StructureRegistryError::InvalidParent)?;
                        if node.children.as_slice() != [child_id]
                            || child.parent != Some(id)
                            || child.role != StructureRole::Span
                            || child
                                .equation_number_binding_v2
                                .as_ref()
                                .map(StructureEquationNumberBindingV2::parent_owner)
                                != Some(semantic.node_id())
                        {
                            return Err(StructureRegistryError::InvalidParent);
                        }
                    }
                    None if !node.children.is_empty() => {
                        return Err(StructureRegistryError::InvalidParent);
                    }
                    None => {}
                }
            }
            StagingStructureSemanticKind::InlineVector { .. } => {
                if node.role != StructureRole::Figure
                    || node.vector_binding_v2.map(StructureVectorBindingV2::kind)
                        != Some(PrecomposedVectorKind::InlineVector)
                {
                    return Err(StructureRegistryError::UnknownSemantic);
                }
            }
            StagingStructureSemanticKind::MathVector { .. } => {
                if node.role != StructureRole::Formula
                    || node.vector_binding_v2.map(StructureVectorBindingV2::kind)
                        != Some(PrecomposedVectorKind::MathVector)
                {
                    return Err(StructureRegistryError::UnknownSemantic);
                }
            }
            StagingStructureSemanticKind::VectorFigure { .. } => {
                if node.role != StructureRole::Figure
                    || node.vector_binding_v2.map(StructureVectorBindingV2::kind)
                        != Some(PrecomposedVectorKind::VectorFigure)
                {
                    return Err(StructureRegistryError::UnknownSemantic);
                }
            }
            StagingStructureSemanticKind::EquationNumber { .. } => {
                if node.role != StructureRole::Span
                    || node.vector_binding_v2.is_some()
                    || node.equation_number_binding_v2.is_none()
                {
                    return Err(StructureRegistryError::UnknownSemantic);
                }
            }
            _ if node.vector_binding_v2.is_some() || node.equation_number_binding_v2.is_some() => {
                return Err(StructureRegistryError::UnknownSemantic);
            }
            _ => {}
        }
    }
    Ok(())
}

impl RegistryBuilder<'_> {
    fn visit_source(
        &mut self,
        source: NodeId,
        parent: Option<StructureNodeId>,
        depth: u32,
    ) -> Result<Option<StructureNodeId>, StructureRegistryError> {
        let record = self
            .semantics
            .record(source)
            .cloned()
            .ok_or(StructureRegistryError::UnknownSemantic)?;
        if !record.kind().creates_structure_element() {
            return Ok(None);
        }
        let (role, alternative, accessible_name, paint_required, actual_text, marker) =
            source_projection(&record)?;
        let id = self.allocate(
            StructureOwner::Source(source),
            record.source_span(),
            role,
            parent,
            record.language().to_owned(),
            alternative,
            accessible_name,
            record.outline_ids().to_vec(),
            paint_required,
            actual_text,
            marker,
            depth,
        )?;
        if self.source_to_structure.insert(source, id).is_some() {
            return Err(StructureRegistryError::InvalidParent);
        }
        self.bind_v2_source_fields(id, &record)?;
        let children = match record.kind() {
            StagingStructureSemanticKind::ListItem { marker } => {
                let label = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::ListLabel,
                    StructureRole::Label,
                    id,
                    record.language(),
                    true,
                    Some(marker.clone()),
                    Some(marker.clone()),
                    depth + 1,
                )?;
                let body = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::ListBody,
                    StructureRole::ListBody,
                    id,
                    record.language(),
                    false,
                    None,
                    None,
                    depth + 1,
                )?;
                let body_children = self.visit_direct_children(source, body, depth + 2)?;
                self.nodes[body.get() as usize].children = body_children;
                vec![label, body]
            }
            StagingStructureSemanticKind::Table { .. } => {
                let source_children = self.direct_children(source)?;
                let mut head_sources = Vec::new();
                let mut body_sources = Vec::new();
                let mut saw_body = false;
                for child in source_children {
                    match child.kind() {
                        StagingStructureSemanticKind::TableRow {
                            section: StagingStructureTableSection::Head,
                            ..
                        } if !saw_body => head_sources.push(child),
                        StagingStructureSemanticKind::TableRow {
                            section: StagingStructureTableSection::Body,
                            ..
                        } => {
                            saw_body = true;
                            body_sources.push(child);
                        }
                        _ => return Err(StructureRegistryError::InvalidParent),
                    }
                }
                let head = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::TableHead,
                    StructureRole::TableHead,
                    id,
                    record.language(),
                    false,
                    None,
                    None,
                    depth + 1,
                )?;
                let head_children = head_sources
                    .into_iter()
                    .map(|child| {
                        self.visit_source(child.node_id(), Some(head), depth + 2)?
                            .ok_or(StructureRegistryError::InvalidParent)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.nodes[head.get() as usize].children = head_children;
                let body = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::TableBody,
                    StructureRole::TableBody,
                    id,
                    record.language(),
                    false,
                    None,
                    None,
                    depth + 1,
                )?;
                let body_children = body_sources
                    .into_iter()
                    .map(|child| {
                        self.visit_source(child.node_id(), Some(body), depth + 2)?
                            .ok_or(StructureRegistryError::InvalidParent)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.nodes[body.get() as usize].children = body_children;
                vec![head, body]
            }
            StagingStructureSemanticKind::Figure { has_caption, .. }
            | StagingStructureSemanticKind::VectorFigure { has_caption, .. }
                if *has_caption =>
            {
                let caption = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::FigureCaption,
                    StructureRole::Caption,
                    id,
                    record.language(),
                    false,
                    None,
                    None,
                    depth + 1,
                )?;
                let caption_children = self.visit_direct_children(source, caption, depth + 2)?;
                self.nodes[caption.get() as usize].children = caption_children;
                vec![caption]
            }
            StagingStructureSemanticKind::FootnoteDefinition { marker, .. } => {
                let label = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::FootnoteLabel,
                    StructureRole::Label,
                    id,
                    record.language(),
                    true,
                    Some(marker.clone()),
                    Some(marker.clone()),
                    depth + 1,
                )?;
                let mut children = vec![label];
                children.extend(self.visit_direct_children(source, id, depth + 1)?);
                children
            }
            StagingStructureSemanticKind::FootnoteReference { marker, .. } => {
                let label = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::FootnoteLabel,
                    StructureRole::Label,
                    id,
                    record.language(),
                    true,
                    Some(marker.clone()),
                    Some(marker.clone()),
                    depth + 1,
                )?;
                vec![label]
            }
            _ => self.visit_direct_children(source, id, depth + 1)?,
        };
        self.nodes[id.get() as usize].children = children;
        self.finish_source_attributes(id, &record)?;
        Ok(Some(id))
    }

    #[allow(clippy::too_many_arguments)]
    fn allocate(
        &mut self,
        owner: StructureOwner,
        source_span: Option<SourceSpan>,
        role: StructureRole,
        parent: Option<StructureNodeId>,
        language: String,
        alternative: Option<String>,
        accessible_name: Option<String>,
        outline_ids: Vec<u32>,
        paint_required: bool,
        actual_text: Option<String>,
        marker: Option<String>,
        depth: u32,
    ) -> Result<StructureNodeId, StructureRegistryError> {
        if depth == 0 || depth > self.limits.get().max_ast_nesting_depth {
            return Err(StructureRegistryError::AstDepthLimit);
        }
        for value in [
            Some(language.as_str()),
            alternative.as_deref(),
            accessible_name.as_deref(),
            actual_text.as_deref(),
            marker.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if u64::try_from(value.len()).map_or(true, |length| {
                length > u64::from(self.limits.get().max_text_buffer_bytes)
            }) {
                return Err(StructureRegistryError::TextLimit);
            }
        }
        if let Some(value) = accessible_name.as_deref() {
            self.derived_text_bytes = self
                .derived_text_bytes
                .checked_add(
                    u64::try_from(value.len())
                        .map_err(|_| StructureRegistryError::TextAggregateLimit)?,
                )
                .ok_or(StructureRegistryError::TextAggregateLimit)?;
            if self.derived_text_bytes > self.limits.get().max_text_bytes {
                return Err(StructureRegistryError::TextAggregateLimit);
            }
        }
        let raw =
            u32::try_from(self.nodes.len()).map_err(|_| StructureRegistryError::AstNodeLimit)?;
        let id = StructureNodeId::new(raw);
        self.nodes
            .try_reserve(1)
            .map_err(|_| StructureRegistryError::AllocationFailure)?;
        self.nodes.push(StructureNodeRecord {
            structure_node_id: id,
            owner,
            source_span,
            role,
            parent,
            children: Vec::new(),
            language,
            language_binding_v2: None,
            vector_binding_v2: None,
            equation_number_binding_v2: None,
            list_numbering: None,
            alternative,
            accessible_name,
            structure_id: None,
            table_attributes: None,
            outline_ids,
            related_nodes: Vec::new(),
            paint_required,
            actual_text,
            marker,
        });
        self.maximum_depth = self.maximum_depth.max(depth);
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    fn allocate_generated(
        &mut self,
        owner_node_id: NodeId,
        slot: GeneratedStructureSlot,
        role: StructureRole,
        parent: StructureNodeId,
        language: &str,
        paint_required: bool,
        actual_text: Option<String>,
        marker: Option<String>,
        depth: u32,
    ) -> Result<StructureNodeId, StructureRegistryError> {
        let key = GeneratedStructureKey::new(owner_node_id, slot);
        let generated_count = self
            .generated
            .len()
            .checked_add(1)
            .ok_or(StructureRegistryError::AstNodeLimit)?;
        let charged_nodes = self
            .semantics
            .records()
            .len()
            .checked_add(generated_count)
            .and_then(|count| u64::try_from(count).ok())
            .ok_or(StructureRegistryError::AstNodeLimit)?;
        if charged_nodes > self.limits.get().max_ast_nodes {
            return Err(StructureRegistryError::AstNodeLimit);
        }
        if !self.generated.insert(key) {
            return Err(StructureRegistryError::InvalidGeneratedNode);
        }
        let id = self.allocate(
            StructureOwner::Generated(key),
            self.semantics
                .record(owner_node_id)
                .and_then(StagingStructureSemanticRecord::source_span),
            role,
            Some(parent),
            language.to_owned(),
            None,
            None,
            Vec::new(),
            paint_required,
            actual_text,
            marker,
            depth,
        )?;
        if self.semantics.is_v2() {
            let parent_binding = self
                .nodes
                .get(parent.get() as usize)
                .and_then(|node| node.language_binding_v2)
                .ok_or(StructureRegistryError::InvalidGeneratedNode)?;
            self.nodes[id.get() as usize].language_binding_v2 = Some(parent_binding);
        }
        Ok(id)
    }

    fn bind_v2_source_fields(
        &mut self,
        id: StructureNodeId,
        source: &StagingStructureSemanticRecord,
    ) -> Result<(), StructureRegistryError> {
        if !self.semantics.is_v2() {
            return Ok(());
        }
        let language_binding = source
            .language_binding_v2()
            .ok_or(StructureRegistryError::UnknownSemantic)?;
        let (vector_binding_v2, equation_number_binding_v2) = match source.kind() {
            StagingStructureSemanticKind::InlineVector {
                metrics_fingerprint,
                ..
            } => (
                Some(StructureVectorBindingV2 {
                    kind: PrecomposedVectorKind::InlineVector,
                    metrics_fingerprint: *metrics_fingerprint,
                }),
                None,
            ),
            StagingStructureSemanticKind::MathVector {
                metrics_fingerprint,
                ..
            } => (
                Some(StructureVectorBindingV2 {
                    kind: PrecomposedVectorKind::MathVector,
                    metrics_fingerprint: *metrics_fingerprint,
                }),
                None,
            ),
            StagingStructureSemanticKind::VectorFigure {
                metrics_fingerprint,
                ..
            } => (
                Some(StructureVectorBindingV2 {
                    kind: PrecomposedVectorKind::VectorFigure,
                    metrics_fingerprint: *metrics_fingerprint,
                }),
                None,
            ),
            StagingStructureSemanticKind::MathVectorBlock {
                metrics_fingerprint,
                ..
            } => (
                Some(StructureVectorBindingV2 {
                    kind: PrecomposedVectorKind::MathVectorBlock,
                    metrics_fingerprint: *metrics_fingerprint,
                }),
                None,
            ),
            StagingStructureSemanticKind::EquationNumber { binding } => (
                None,
                Some(StructureEquationNumberBindingV2 {
                    parent_owner: binding.parent_owner(),
                    text_span: binding.text_span(),
                    text_buffer_sha256: binding.text_buffer_sha256(),
                    exact_text: binding.exact_text().to_owned(),
                    exact_text_sha256: binding.exact_text_sha256(),
                }),
            ),
            _ => (None, None),
        };
        let node = self
            .nodes
            .get_mut(id.get() as usize)
            .ok_or(StructureRegistryError::InvalidParent)?;
        node.language_binding_v2 = Some(language_binding);
        node.vector_binding_v2 = vector_binding_v2;
        node.equation_number_binding_v2 = equation_number_binding_v2;
        Ok(())
    }

    fn direct_children(
        &self,
        source: NodeId,
    ) -> Result<Vec<StagingStructureSemanticRecord>, StructureRegistryError> {
        let mut ordinary = self
            .semantics
            .records()
            .iter()
            .filter(|record| {
                record.parent_node_id() == Some(source)
                    && record.insertion_after_node_id().is_none()
            })
            .cloned()
            .collect::<Vec<_>>();
        ordinary.sort_by_key(StagingStructureSemanticRecord::node_id);
        let mut inserted = BTreeMap::<NodeId, Vec<StagingStructureSemanticRecord>>::new();
        for record in self.semantics.records().iter().filter(|record| {
            record.parent_node_id() == Some(source) && record.insertion_after_node_id().is_some()
        }) {
            inserted
                .entry(
                    record
                        .insertion_after_node_id()
                        .ok_or(StructureRegistryError::InvalidParent)?,
                )
                .or_default()
                .push(record.clone());
        }
        for values in inserted.values_mut() {
            if values.iter().any(|record| {
                !matches!(
                    record.kind(),
                    StagingStructureSemanticKind::FootnoteDefinition {
                        reference_node_ids,
                        ..
                    } if !reference_node_ids.is_empty()
                )
            }) {
                return Err(StructureRegistryError::InvalidParent);
            }
            values.sort_by_key(|record| {
                let last_reference = match record.kind() {
                    StagingStructureSemanticKind::FootnoteDefinition {
                        reference_node_ids, ..
                    } => reference_node_ids
                        .iter()
                        .max()
                        .copied()
                        .unwrap_or(NodeId::new(0)),
                    _ => NodeId::new(0),
                };
                (last_reference, record.node_id())
            });
        }
        let mut output = Vec::new();
        for record in ordinary {
            let node_id = record.node_id();
            output.push(record);
            if let Some(mut notes) = inserted.remove(&node_id) {
                output.append(&mut notes);
            }
        }
        if !inserted.is_empty() {
            return Err(StructureRegistryError::InvalidParent);
        }
        Ok(output)
    }

    fn visit_direct_children(
        &mut self,
        source: NodeId,
        parent: StructureNodeId,
        depth: u32,
    ) -> Result<Vec<StructureNodeId>, StructureRegistryError> {
        let mut output = Vec::new();
        for child in self.direct_children(source)? {
            if let Some(child_id) = self.visit_source(child.node_id(), Some(parent), depth)? {
                output.push(child_id);
            }
        }
        Ok(output)
    }

    fn finish_source_attributes(
        &mut self,
        id: StructureNodeId,
        source: &StagingStructureSemanticRecord,
    ) -> Result<(), StructureRegistryError> {
        let structure_id = match source.kind() {
            StagingStructureSemanticKind::FootnoteDefinition { .. }
            | StagingStructureSemanticKind::TableCell {
                section: StagingStructureTableSection::Head,
                ..
            } => Some(format!("typaxis-se-{:08x}", id.get())),
            _ => None,
        };
        let list_numbering = match source.kind() {
            StagingStructureSemanticKind::List { ordered } => Some(if *ordered {
                StructureListNumbering::Decimal
            } else {
                StructureListNumbering::Disc
            }),
            _ => None,
        };
        let table_attributes = match source.kind() {
            StagingStructureSemanticKind::TableCell {
                section,
                row_ordinal,
                column_ordinal,
                colspan,
                rowspan,
                header_node_ids,
                ..
            } => {
                let header_ids = header_node_ids
                    .iter()
                    .map(|source_id| {
                        self.source_to_structure
                            .get(source_id)
                            .and_then(|id| self.nodes.get(id.get() as usize))
                            .and_then(|node| node.structure_id.clone())
                            .ok_or(StructureRegistryError::InvalidTableHeader)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if *section == StagingStructureTableSection::Body && header_ids.is_empty() {
                    return Err(StructureRegistryError::InvalidTableHeader);
                }
                Some(StructureTableAttributes {
                    section: *section,
                    row_ordinal: *row_ordinal,
                    column_ordinal: *column_ordinal,
                    colspan: *colspan,
                    rowspan: *rowspan,
                    header_ids,
                })
            }
            _ => None,
        };
        for value in structure_id.as_deref().into_iter().chain(
            table_attributes
                .as_ref()
                .into_iter()
                .flat_map(|attributes| attributes.header_ids.iter().map(String::as_str)),
        ) {
            if u64::try_from(value.len()).map_or(true, |length| {
                length > u64::from(self.limits.get().max_text_buffer_bytes)
            }) {
                return Err(StructureRegistryError::TextLimit);
            }
        }
        let node = self
            .nodes
            .get_mut(id.get() as usize)
            .ok_or(StructureRegistryError::InvalidParent)?;
        node.structure_id = structure_id;
        node.list_numbering = list_numbering;
        node.table_attributes = table_attributes;
        Ok(())
    }

    fn close_footnote_relations(&mut self) -> Result<(), StructureRegistryError> {
        let mut definitions = Vec::new();
        let mut reference_to_note = BTreeMap::new();
        for definition in self.semantics.records() {
            let StagingStructureSemanticKind::FootnoteDefinition {
                footnote_id,
                reference_node_ids,
                ..
            } = definition.kind()
            else {
                continue;
            };
            let note_id = self
                .source_to_structure
                .get(&definition.node_id())
                .copied()
                .ok_or(StructureRegistryError::InvalidParent)?;
            let references = reference_node_ids
                .iter()
                .map(|source_id| {
                    let source = self
                        .semantics
                        .record(*source_id)
                        .ok_or(StructureRegistryError::InvalidParent)?;
                    if !matches!(
                        source.kind(),
                        StagingStructureSemanticKind::FootnoteReference {
                            footnote_id: reference_footnote_id,
                            ..
                        } if reference_footnote_id == footnote_id
                    ) {
                        return Err(StructureRegistryError::InvalidParent);
                    }
                    let reference_id = self
                        .source_to_structure
                        .get(source_id)
                        .copied()
                        .ok_or(StructureRegistryError::InvalidParent)?;
                    if reference_to_note.insert(reference_id, note_id).is_some() {
                        return Err(StructureRegistryError::InvalidParent);
                    }
                    Ok(reference_id)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if references.is_empty() {
                return Err(StructureRegistryError::InvalidParent);
            }
            definitions.push((note_id, references));
        }
        let reference_count = self
            .semantics
            .records()
            .iter()
            .filter(|record| {
                matches!(
                    record.kind(),
                    StagingStructureSemanticKind::FootnoteReference { .. }
                )
            })
            .count();
        if reference_to_note.len() != reference_count {
            return Err(StructureRegistryError::InvalidParent);
        }
        for (note_id, references) in definitions {
            self.nodes[note_id.get() as usize].related_nodes = references;
        }
        for (reference_id, note_id) in reference_to_note {
            self.nodes[reference_id.get() as usize].related_nodes = vec![note_id];
        }
        Ok(())
    }
}

type SourceProjection = (
    StructureRole,
    Option<String>,
    Option<String>,
    bool,
    Option<String>,
    Option<String>,
);

fn source_projection(
    record: &StagingStructureSemanticRecord,
) -> Result<SourceProjection, StructureRegistryError> {
    let value = match record.kind() {
        StagingStructureSemanticKind::Document => {
            (StructureRole::Document, None, None, false, None, None)
        }
        StagingStructureSemanticKind::SemanticContainer { semantic_kind } => {
            let role = match semantic_kind.as_str() {
                "result" => StructureRole::Result,
                "proof" => StructureRole::Proof,
                "exercise" => StructureRole::Exercise,
                _ => return Err(StructureRegistryError::UnknownSemantic),
            };
            (role, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Paragraph { .. } => {
            (StructureRole::Paragraph, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Heading { level, .. } => {
            let role = match level {
                1 => StructureRole::Heading1,
                2 => StructureRole::Heading2,
                3 => StructureRole::Heading3,
                4 => StructureRole::Heading4,
                5 => StructureRole::Heading5,
                6 => StructureRole::Heading6,
                _ => return Err(StructureRegistryError::UnknownSemantic),
            };
            (role, None, None, false, None, None)
        }
        StagingStructureSemanticKind::List { .. } => {
            (StructureRole::List, None, None, false, None, None)
        }
        StagingStructureSemanticKind::ListItem { .. } => {
            (StructureRole::ListItem, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Table { .. } => {
            (StructureRole::Table, None, None, false, None, None)
        }
        StagingStructureSemanticKind::TableRow { .. } => {
            (StructureRole::TableRow, None, None, false, None, None)
        }
        StagingStructureSemanticKind::TableCell { section, .. } => {
            let role = if *section == StagingStructureTableSection::Head {
                StructureRole::TableHeader
            } else {
                StructureRole::TableData
            };
            (role, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Figure { alternative, .. } => (
            StructureRole::Figure,
            Some(alternative.clone()),
            None,
            true,
            None,
            None,
        ),
        StagingStructureSemanticKind::DisplayMath { alternative }
        | StagingStructureSemanticKind::InlineMath { alternative } => (
            StructureRole::Formula,
            Some(alternative.clone()),
            None,
            true,
            Some(alternative.clone()),
            None,
        ),
        StagingStructureSemanticKind::InlineVector {
            alternative,
            authored_actual_text,
            ..
        } => (
            StructureRole::Figure,
            Some(alternative.clone()),
            None,
            true,
            authored_actual_text.clone(),
            None,
        ),
        StagingStructureSemanticKind::MathVector {
            alternative,
            resolved_actual_text,
            ..
        }
        | StagingStructureSemanticKind::MathVectorBlock {
            alternative,
            resolved_actual_text,
            ..
        } => (
            StructureRole::Formula,
            Some(alternative.clone()),
            None,
            true,
            Some(resolved_actual_text.clone()),
            None,
        ),
        StagingStructureSemanticKind::VectorFigure { alternative, .. } => (
            StructureRole::Figure,
            Some(alternative.clone()),
            None,
            true,
            None,
            None,
        ),
        StagingStructureSemanticKind::EquationNumber { .. } => {
            (StructureRole::Span, None, None, true, None, None)
        }
        StagingStructureSemanticKind::FootnoteDefinition { .. } => {
            (StructureRole::Note, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Text { text } => (
            StructureRole::Span,
            None,
            None,
            true,
            Some(text.clone()),
            None,
        ),
        StagingStructureSemanticKind::Emphasis => {
            (StructureRole::Emphasis, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Strong => {
            (StructureRole::Strong, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Link { accessible_name } => (
            StructureRole::Link,
            None,
            Some(accessible_name.clone()),
            false,
            None,
            None,
        ),
        StagingStructureSemanticKind::Reference { label } => (
            StructureRole::Reference,
            None,
            None,
            true,
            Some(label.clone()),
            Some(label.clone()),
        ),
        StagingStructureSemanticKind::FootnoteReference { .. } => {
            (StructureRole::Reference, None, None, false, None, None)
        }
        StagingStructureSemanticKind::PageBreak
        | StagingStructureSemanticKind::Anchor
        | StagingStructureSemanticKind::SoftBreak
        | StagingStructureSemanticKind::HardBreak => {
            return Err(StructureRegistryError::UnknownSemantic)
        }
    };
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StructureArtifactClass {
    Pagination,
    PaginationHeader,
    PaginationFooter,
    Layout,
}

impl StructureArtifactClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pagination => "pagination",
            Self::PaginationHeader => "pagination_header",
            Self::PaginationFooter => "pagination_footer",
            Self::Layout => "layout",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectedStructurePaintOwner {
    Structure(StructureNodeId),
    Artifact {
        class: StructureArtifactClass,
        occurrence: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedStructurePage {
    pub page_index: u32,
    pub width_raw: i64,
    pub height_raw: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedStructurePaintInput {
    pub selected_paint_id: u32,
    pub page_index: u32,
    pub paint_ordinal: u32,
    pub semantic_fragment_ordinal: u32,
    pub owner: SelectedStructurePaintOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedVectorPaintBindingV2 {
    pub usage_id: u32,
    pub kind: PrecomposedVectorKind,
    pub metrics_fingerprint: [u8; 32],
    pub display_command_fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedEquationNumberPaintBindingV2 {
    pub parent_owner: NodeId,
    pub text_span: TextSpan,
    pub text_buffer_sha256: [u8; 32],
    pub exact_text: String,
    pub exact_text_sha256: [u8; 32],
    pub shape_fingerprint: [u8; 32],
    pub glyph_receipt_fingerprint: [u8; 32],
    pub shape_language_fingerprint: [u8; 32],
    pub language_record_fingerprint: [u8; 32],
    pub parent_language_record_fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectedStructurePaintBindingV2 {
    Standard,
    Vector(SelectedVectorPaintBindingV2),
    EquationNumber(SelectedEquationNumberPaintBindingV2),
}

impl SelectedStructurePaintBindingV2 {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Vector(_) => "vector",
            Self::EquationNumber(_) => "equation_number",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedStructurePaintInputV2 {
    pub selected_paint_id: u32,
    pub page_index: u32,
    pub paint_ordinal: u32,
    pub semantic_fragment_ordinal: u32,
    pub owner: SelectedStructurePaintOwner,
    pub binding: SelectedStructurePaintBindingV2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedStructureAnnotationInput {
    pub annotation_id: u32,
    pub page_index: u32,
    pub annotation_ordinal: u32,
    pub owner_node_id: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedStructurePaint {
    selected_paint_id: u32,
    page_index: u32,
    paint_ordinal: u32,
    semantic_fragment_ordinal: u32,
    owner: SelectedStructurePaintOwner,
    role: Option<StructureRole>,
    language: Option<String>,
    actual_text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedStructurePaintV2 {
    selected_paint_id: u32,
    page_index: u32,
    paint_ordinal: u32,
    semantic_fragment_ordinal: u32,
    owner: SelectedStructurePaintOwner,
    role: Option<StructureRole>,
    language: Option<String>,
    actual_text: Option<String>,
    binding: SelectedStructurePaintBindingV2,
}

impl SelectedStructurePaintV2 {
    pub const fn selected_paint_id(&self) -> u32 {
        self.selected_paint_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub const fn semantic_fragment_ordinal(&self) -> u32 {
        self.semantic_fragment_ordinal
    }
    pub const fn owner(&self) -> SelectedStructurePaintOwner {
        self.owner
    }
    pub const fn role(&self) -> Option<StructureRole> {
        self.role
    }
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }
    pub const fn binding(&self) -> &SelectedStructurePaintBindingV2 {
        &self.binding
    }
}

impl SelectedStructurePaint {
    pub const fn selected_paint_id(&self) -> u32 {
        self.selected_paint_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub const fn semantic_fragment_ordinal(&self) -> u32 {
        self.semantic_fragment_ordinal
    }
    pub const fn owner(&self) -> SelectedStructurePaintOwner {
        self.owner
    }
    pub const fn role(&self) -> Option<StructureRole> {
        self.role
    }
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedStructureAnnotation {
    annotation_id: u32,
    page_index: u32,
    annotation_ordinal: u32,
    owner_node_id: NodeId,
    structure_node_id: StructureNodeId,
    accessible_name: String,
}

impl SelectedStructureAnnotation {
    pub const fn annotation_id(&self) -> u32 {
        self.annotation_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn annotation_ordinal(&self) -> u32 {
        self.annotation_ordinal
    }
    pub const fn owner_node_id(&self) -> NodeId {
        self.owner_node_id
    }
    pub const fn structure_node_id(&self) -> StructureNodeId {
        self.structure_node_id
    }
    pub fn accessible_name(&self) -> &str {
        &self.accessible_name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedStructureBindingReceipt {
    structure_registry_sha256: [u8; 32],
    authorization_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: Vec<SelectedStructurePage>,
    paints: Vec<SelectedStructurePaint>,
    annotations: Vec<SelectedStructureAnnotation>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl SelectedStructureBindingReceipt {
    pub const fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }
    pub const fn authorization_sha256(&self) -> [u8; 32] {
        self.authorization_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn selected_layout_fragment_count(&self) -> u64 {
        self.selected_layout_fragment_count
    }
    pub fn pages(&self) -> &[SelectedStructurePage] {
        &self.pages
    }
    pub fn paints(&self) -> &[SelectedStructurePaint] {
        &self.paints
    }
    pub fn annotations(&self) -> &[SelectedStructureAnnotation] {
        &self.annotations
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        registry: &StructureRegistryReceipt,
        authorization: &StagingAccessibilityProfileAuthorization,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), SelectedStructureBindingError> {
        registry
            .verify_sealed(authorization, limits)
            .map_err(|_| SelectedStructureBindingError::RegistryMismatch)?;
        let observed = select_structure_bindings_inner(
            registry,
            authorization,
            limits,
            self.selected_layout_sha256,
            self.selected_layout_fragment_count,
            &self.pages,
            &self
                .paints
                .iter()
                .map(|paint| SelectedStructurePaintInput {
                    selected_paint_id: paint.selected_paint_id,
                    page_index: paint.page_index,
                    paint_ordinal: paint.paint_ordinal,
                    semantic_fragment_ordinal: paint.semantic_fragment_ordinal,
                    owner: paint.owner,
                })
                .collect::<Vec<_>>(),
            &self
                .annotations
                .iter()
                .map(|annotation| SelectedStructureAnnotationInput {
                    annotation_id: annotation.annotation_id,
                    page_index: annotation.page_index,
                    annotation_ordinal: annotation.annotation_ordinal,
                    owner_node_id: annotation.owner_node_id,
                })
                .collect::<Vec<_>>(),
        )?;
        if self != &observed {
            return Err(SelectedStructureBindingError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedStructureBindingReceiptV2 {
    structure_registry_sha256: [u8; 32],
    authorization_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: Vec<SelectedStructurePage>,
    paints: Vec<SelectedStructurePaintV2>,
    annotations: Vec<SelectedStructureAnnotation>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl SelectedStructureBindingReceiptV2 {
    pub const fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }
    pub const fn authorization_sha256(&self) -> [u8; 32] {
        self.authorization_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn selected_layout_fragment_count(&self) -> u64 {
        self.selected_layout_fragment_count
    }
    pub fn pages(&self) -> &[SelectedStructurePage] {
        &self.pages
    }
    pub fn paints(&self) -> &[SelectedStructurePaintV2] {
        &self.paints
    }
    pub fn annotations(&self) -> &[SelectedStructureAnnotation] {
        &self.annotations
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        registry: &StructureRegistryReceiptV2,
        authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), SelectedStructureBindingError> {
        registry
            .verify_sealed(authorization, limits)
            .map_err(|_| SelectedStructureBindingError::RegistryMismatch)?;
        let inputs = self
            .paints
            .iter()
            .map(|paint| SelectedStructurePaintInputV2 {
                selected_paint_id: paint.selected_paint_id,
                page_index: paint.page_index,
                paint_ordinal: paint.paint_ordinal,
                semantic_fragment_ordinal: paint.semantic_fragment_ordinal,
                owner: paint.owner,
                binding: paint.binding.clone(),
            })
            .collect::<Vec<_>>();
        let annotations = self
            .annotations
            .iter()
            .map(|annotation| SelectedStructureAnnotationInput {
                annotation_id: annotation.annotation_id,
                page_index: annotation.page_index,
                annotation_ordinal: annotation.annotation_ordinal,
                owner_node_id: annotation.owner_node_id,
            })
            .collect::<Vec<_>>();
        let observed = select_structure_bindings_v2_inner(
            registry,
            authorization,
            limits,
            self.selected_layout_sha256,
            self.selected_layout_fragment_count,
            &self.pages,
            &inputs,
            &annotations,
        )?;
        if self != &observed {
            return Err(SelectedStructureBindingError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedStructureBindingError {
    RegistryMismatch,
    NonCanonicalPage,
    MissingPaint,
    ExtraPaint,
    PaintOrder,
    InvalidArtifact,
    InvalidAnnotation,
    InvalidVector,
    InvalidEquationNumber,
    FragmentLimit,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for SelectedStructureBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistryMismatch => {
                formatter.write_str("I9190: selected structure registry mismatch")
            }
            Self::NonCanonicalPage => {
                formatter.write_str("L5100: selected structure page mismatch")
            }
            Self::MissingPaint => formatter.write_str("I9190: required structure paint is missing"),
            Self::ExtraPaint => formatter.write_str("I9190: selected structure paint has no owner"),
            Self::PaintOrder => {
                formatter.write_str("I9190: selected structure paint order mismatch")
            }
            Self::InvalidArtifact => {
                formatter.write_str("I9190: selected artifact classification mismatch")
            }
            Self::InvalidAnnotation => {
                formatter.write_str("I9190: selected Link annotation mismatch")
            }
            Self::InvalidVector => {
                formatter.write_str("I9190: selected vector structure binding mismatch")
            }
            Self::InvalidEquationNumber => {
                formatter.write_str("I9190: selected equation-number structure binding mismatch")
            }
            Self::FragmentLimit => {
                formatter.write_str("L5110: marked-content fragment limit exceeded")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5110: selected structure allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: selected structure receipt mismatch")
            }
        }
    }
}

impl std::error::Error for SelectedStructureBindingError {}

#[allow(clippy::too_many_arguments)]
pub fn select_structure_bindings(
    registry: &StructureRegistryReceipt,
    authorization: &StagingAccessibilityProfileAuthorization,
    limits: &ValidatedResourceLimits,
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: &[SelectedStructurePage],
    paints: &[SelectedStructurePaintInput],
    annotations: &[SelectedStructureAnnotationInput],
) -> Result<SelectedStructureBindingReceipt, SelectedStructureBindingError> {
    registry
        .verify_sealed(authorization, limits)
        .map_err(|_| SelectedStructureBindingError::RegistryMismatch)?;
    select_structure_bindings_inner(
        registry,
        authorization,
        limits,
        selected_layout_sha256,
        selected_layout_fragment_count,
        pages,
        paints,
        annotations,
    )
}

#[allow(clippy::too_many_arguments)]
fn select_structure_bindings_inner(
    registry: &StructureRegistryReceipt,
    authorization: &StagingAccessibilityProfileAuthorization,
    limits: &ValidatedResourceLimits,
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: &[SelectedStructurePage],
    paints: &[SelectedStructurePaintInput],
    annotations: &[SelectedStructureAnnotationInput],
) -> Result<SelectedStructureBindingReceipt, SelectedStructureBindingError> {
    validate_selected_pages(pages, limits)?;
    if selected_layout_fragment_count > limits.get().max_fragments {
        return Err(SelectedStructureBindingError::FragmentLimit);
    }
    let required = registry
        .nodes()
        .iter()
        .filter(|node| node.paint_required())
        .map(StructureNodeRecord::structure_node_id)
        .collect::<BTreeSet<_>>();
    let mut observed_required = BTreeSet::new();
    let mut observed_fragments = BTreeMap::<StructureNodeId, BTreeSet<u32>>::new();
    let mut artifact_occurrences = BTreeMap::<StructureArtifactClass, BTreeSet<u32>>::new();
    let mut closed_groups = BTreeSet::<(SelectedStructurePaintOwner, u32)>::new();
    let mut previous_group = None;
    let mut previous_paint = None;
    let mut next_page_paint = BTreeMap::<u32, u32>::new();
    let mut selected_paints = Vec::new();
    selected_paints
        .try_reserve_exact(paints.len())
        .map_err(|_| SelectedStructureBindingError::AllocationFailure)?;
    for (index, paint) in paints.iter().enumerate() {
        let paint_order = (paint.page_index, paint.paint_ordinal);
        if usize::try_from(paint.selected_paint_id) != Ok(index)
            || pages.get(paint.page_index as usize).is_none()
            || *next_page_paint.entry(paint.page_index).or_insert(0) != paint.paint_ordinal
            || previous_paint.is_some_and(|previous| previous >= paint_order)
        {
            return Err(SelectedStructureBindingError::PaintOrder);
        }
        previous_paint = Some(paint_order);
        *next_page_paint
            .get_mut(&paint.page_index)
            .ok_or(SelectedStructureBindingError::PaintOrder)? += 1;
        let group = (paint.owner, paint.semantic_fragment_ordinal);
        let page_group = (paint.page_index, group);
        if previous_group != Some(page_group) {
            if let Some((_, previous)) = previous_group.replace(page_group) {
                closed_groups.insert(previous);
            }
            if closed_groups.contains(&group) {
                return Err(SelectedStructureBindingError::PaintOrder);
            }
        }
        let (role, language, actual_text) = match paint.owner {
            SelectedStructurePaintOwner::Structure(owner) => {
                let node = registry
                    .node(owner)
                    .ok_or(SelectedStructureBindingError::ExtraPaint)?;
                if !node.paint_required() {
                    return Err(SelectedStructureBindingError::ExtraPaint);
                }
                observed_fragments
                    .entry(owner)
                    .or_default()
                    .insert(paint.semantic_fragment_ordinal);
                observed_required.insert(owner);
                (
                    Some(node.role()),
                    (node.language()
                        != registry
                            .nodes()
                            .first()
                            .map_or("", StructureNodeRecord::language))
                    .then(|| node.language().to_owned()),
                    node.actual_text().map(str::to_owned),
                )
            }
            SelectedStructurePaintOwner::Artifact { class, occurrence } => {
                if !matches!(
                    class,
                    StructureArtifactClass::Pagination
                        | StructureArtifactClass::PaginationHeader
                        | StructureArtifactClass::PaginationFooter
                        | StructureArtifactClass::Layout
                ) || paint.semantic_fragment_ordinal != 0
                {
                    return Err(SelectedStructureBindingError::InvalidArtifact);
                }
                artifact_occurrences
                    .entry(class)
                    .or_default()
                    .insert(occurrence);
                (None, None, None)
            }
        };
        selected_paints.push(SelectedStructurePaint {
            selected_paint_id: paint.selected_paint_id,
            page_index: paint.page_index,
            paint_ordinal: paint.paint_ordinal,
            semantic_fragment_ordinal: paint.semantic_fragment_ordinal,
            owner: paint.owner,
            role,
            language,
            actual_text,
        });
    }
    if observed_required != required {
        return Err(SelectedStructureBindingError::MissingPaint);
    }
    if let Some((_, previous)) = previous_group {
        closed_groups.insert(previous);
    }
    for fragments in observed_fragments.values() {
        let count = u32::try_from(fragments.len())
            .map_err(|_| SelectedStructureBindingError::FragmentLimit)?;
        if !fragments.iter().copied().eq(0..count) {
            return Err(SelectedStructureBindingError::PaintOrder);
        }
    }
    for occurrences in artifact_occurrences.values() {
        let count = u32::try_from(occurrences.len())
            .map_err(|_| SelectedStructureBindingError::FragmentLimit)?;
        if !occurrences.iter().copied().eq(0..count) {
            return Err(SelectedStructureBindingError::InvalidArtifact);
        }
    }
    if u64::try_from(closed_groups.len())
        .map_or(true, |count| count > selected_layout_fragment_count)
    {
        return Err(SelectedStructureBindingError::FragmentLimit);
    }
    for node in registry.nodes().iter().filter(|node| {
        node.paint_required()
            && matches!(
                node.role(),
                StructureRole::Figure
                    | StructureRole::Formula
                    | StructureRole::Label
                    | StructureRole::Reference
            )
    }) {
        if observed_fragments
            .get(&node.structure_node_id())
            .map_or(true, |fragments| {
                fragments.len() != 1 || !fragments.contains(&0)
            })
        {
            return Err(SelectedStructureBindingError::PaintOrder);
        }
    }
    let link_nodes = registry
        .nodes()
        .iter()
        .filter(|node| node.role() == StructureRole::Link)
        .map(|node| match node.owner() {
            StructureOwner::Source(source) => Ok((source, node)),
            StructureOwner::Generated(_) => Err(SelectedStructureBindingError::InvalidAnnotation),
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut linked = BTreeSet::new();
    let mut next_annotation = BTreeMap::<u32, u32>::new();
    let mut previous_annotation = None;
    let mut selected_annotations = Vec::new();
    selected_annotations
        .try_reserve_exact(annotations.len())
        .map_err(|_| SelectedStructureBindingError::AllocationFailure)?;
    for (index, annotation) in annotations.iter().enumerate() {
        let node = link_nodes
            .get(&annotation.owner_node_id)
            .ok_or(SelectedStructureBindingError::InvalidAnnotation)?;
        let order_key = (annotation.page_index, annotation.annotation_ordinal);
        if usize::try_from(annotation.annotation_id) != Ok(index)
            || pages.get(annotation.page_index as usize).is_none()
            || *next_annotation.entry(annotation.page_index).or_insert(0)
                != annotation.annotation_ordinal
            || previous_annotation.is_some_and(|previous| previous >= order_key)
        {
            return Err(SelectedStructureBindingError::InvalidAnnotation);
        }
        previous_annotation = Some(order_key);
        *next_annotation
            .get_mut(&annotation.page_index)
            .ok_or(SelectedStructureBindingError::InvalidAnnotation)? += 1;
        linked.insert(annotation.owner_node_id);
        selected_annotations.push(SelectedStructureAnnotation {
            annotation_id: annotation.annotation_id,
            page_index: annotation.page_index,
            annotation_ordinal: annotation.annotation_ordinal,
            owner_node_id: annotation.owner_node_id,
            structure_node_id: node.structure_node_id(),
            accessible_name: node
                .accessible_name()
                .ok_or(SelectedStructureBindingError::InvalidAnnotation)?
                .to_owned(),
        });
    }
    if linked != link_nodes.keys().copied().collect() {
        return Err(SelectedStructureBindingError::InvalidAnnotation);
    }
    let mut receipt = SelectedStructureBindingReceipt {
        structure_registry_sha256: registry.fingerprint(),
        authorization_sha256: authorization.fingerprint(),
        limits_sha256: limits_fingerprint(limits),
        selected_layout_sha256,
        selected_layout_fragment_count,
        pages: pages.to_vec(),
        paints: selected_paints,
        annotations: selected_annotations,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_selected(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    Ok(receipt)
}

#[allow(clippy::too_many_arguments)]
pub fn select_structure_bindings_v2(
    registry: &StructureRegistryReceiptV2,
    authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: &[SelectedStructurePage],
    paints: &[SelectedStructurePaintInputV2],
    annotations: &[SelectedStructureAnnotationInput],
) -> Result<SelectedStructureBindingReceiptV2, SelectedStructureBindingError> {
    registry
        .verify_sealed(authorization, limits)
        .map_err(|_| SelectedStructureBindingError::RegistryMismatch)?;
    select_structure_bindings_v2_inner(
        registry,
        authorization,
        limits,
        selected_layout_sha256,
        selected_layout_fragment_count,
        pages,
        paints,
        annotations,
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn select_structure_bindings_v2_inner(
    registry: &StructureRegistryReceiptV2,
    authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: &[SelectedStructurePage],
    paints: &[SelectedStructurePaintInputV2],
    annotations: &[SelectedStructureAnnotationInput],
) -> Result<SelectedStructureBindingReceiptV2, SelectedStructureBindingError> {
    validate_selected_pages(pages, limits.base())?;
    if selected_layout_sha256 == [0; 32] {
        return Err(SelectedStructureBindingError::ReceiptMismatch);
    }
    if selected_layout_fragment_count > limits.base().get().max_fragments {
        return Err(SelectedStructureBindingError::FragmentLimit);
    }
    let required = registry
        .nodes()
        .iter()
        .filter(|node| node.paint_required())
        .map(StructureNodeRecord::structure_node_id)
        .collect::<BTreeSet<_>>();
    let mut observed_required = BTreeSet::new();
    let mut observed_fragments = BTreeMap::<StructureNodeId, BTreeSet<u32>>::new();
    let mut artifact_occurrences = BTreeMap::<StructureArtifactClass, BTreeSet<u32>>::new();
    let mut closed_groups = BTreeSet::<(SelectedStructurePaintOwner, u32)>::new();
    let mut previous_group = None;
    let mut previous_paint = None;
    let mut previous_page = None;
    let mut vector_usage_ids = BTreeSet::new();
    let mut vector_structure_nodes = BTreeSet::new();
    let mut selected_paints = Vec::new();
    selected_paints
        .try_reserve_exact(paints.len())
        .map_err(|_| SelectedStructureBindingError::AllocationFailure)?;
    for (index, paint) in paints.iter().enumerate() {
        let paint_order = (paint.page_index, paint.paint_ordinal);
        // Upstream block layout may reserve an ordinal for a structural slot
        // that emits no paint (for example, an empty caption). Keep physical
        // ordinals strictly ordered and page-zero-based; the marked-content
        // finalizer assigns its own dense MCID sequence independently.
        if usize::try_from(paint.selected_paint_id) != Ok(index)
            || pages.get(paint.page_index as usize).is_none()
            || previous_paint.is_some_and(|previous| previous >= paint_order)
            || (previous_page != Some(paint.page_index) && paint.paint_ordinal != 0)
        {
            return Err(SelectedStructureBindingError::PaintOrder);
        }
        let preceding = selected_paints.last();
        previous_paint = Some(paint_order);
        previous_page = Some(paint.page_index);
        let group = (paint.owner, paint.semantic_fragment_ordinal);
        let page_group = (paint.page_index, group);
        if previous_group != Some(page_group) {
            if let Some((_, previous)) = previous_group.replace(page_group) {
                closed_groups.insert(previous);
            }
            if closed_groups.contains(&group) {
                return Err(SelectedStructureBindingError::PaintOrder);
            }
        }
        let (role, language, actual_text) = match paint.owner {
            SelectedStructurePaintOwner::Structure(owner) => {
                let node = registry
                    .node(owner)
                    .ok_or(SelectedStructureBindingError::ExtraPaint)?;
                if !node.paint_required() {
                    return Err(SelectedStructureBindingError::ExtraPaint);
                }
                validate_selected_paint_binding_v2(
                    registry,
                    node,
                    paint,
                    preceding,
                    &mut vector_usage_ids,
                    &mut vector_structure_nodes,
                )?;
                observed_fragments
                    .entry(owner)
                    .or_default()
                    .insert(paint.semantic_fragment_ordinal);
                observed_required.insert(owner);
                (
                    Some(node.role()),
                    (node.language()
                        != registry
                            .nodes()
                            .first()
                            .map_or("", StructureNodeRecord::language))
                    .then(|| node.language().to_owned()),
                    node.actual_text().map(str::to_owned),
                )
            }
            SelectedStructurePaintOwner::Artifact { class, occurrence } => {
                if !matches!(paint.binding, SelectedStructurePaintBindingV2::Standard)
                    || !matches!(
                        class,
                        StructureArtifactClass::Pagination
                            | StructureArtifactClass::PaginationHeader
                            | StructureArtifactClass::PaginationFooter
                            | StructureArtifactClass::Layout
                    )
                    || paint.semantic_fragment_ordinal != 0
                {
                    return Err(SelectedStructureBindingError::InvalidArtifact);
                }
                artifact_occurrences
                    .entry(class)
                    .or_default()
                    .insert(occurrence);
                (None, None, None)
            }
        };
        selected_paints.push(SelectedStructurePaintV2 {
            selected_paint_id: paint.selected_paint_id,
            page_index: paint.page_index,
            paint_ordinal: paint.paint_ordinal,
            semantic_fragment_ordinal: paint.semantic_fragment_ordinal,
            owner: paint.owner,
            role,
            language,
            actual_text,
            binding: paint.binding.clone(),
        });
    }
    if observed_required != required {
        return Err(SelectedStructureBindingError::MissingPaint);
    }
    if let Some((_, previous)) = previous_group {
        closed_groups.insert(previous);
    }
    for fragments in observed_fragments.values() {
        let count = u32::try_from(fragments.len())
            .map_err(|_| SelectedStructureBindingError::FragmentLimit)?;
        if !fragments.iter().copied().eq(0..count) {
            return Err(SelectedStructureBindingError::PaintOrder);
        }
    }
    for occurrences in artifact_occurrences.values() {
        let count = u32::try_from(occurrences.len())
            .map_err(|_| SelectedStructureBindingError::FragmentLimit)?;
        if !occurrences.iter().copied().eq(0..count) {
            return Err(SelectedStructureBindingError::InvalidArtifact);
        }
    }
    let vector_usage_count = u32::try_from(vector_usage_ids.len())
        .map_err(|_| SelectedStructureBindingError::FragmentLimit)?;
    let equation_number_count = selected_paints
        .iter()
        .filter(|paint| {
            matches!(
                paint.binding(),
                SelectedStructurePaintBindingV2::EquationNumber(_)
            )
        })
        .count();
    let maximum_structure_groups = usize::try_from(selected_layout_fragment_count)
        .ok()
        .and_then(|count| count.checked_add(equation_number_count))
        .ok_or(SelectedStructureBindingError::FragmentLimit)?;
    if !vector_usage_ids.iter().copied().eq(0..vector_usage_count)
        || closed_groups.len() > maximum_structure_groups
        || u64::try_from(closed_groups.len())
            .map_or(true, |count| count > limits.base().get().max_fragments)
    {
        return Err(SelectedStructureBindingError::PaintOrder);
    }
    for node in registry.nodes().iter().filter(|node| {
        node.paint_required()
            && (node.vector_binding_v2().is_some()
                || node.equation_number_binding_v2().is_some()
                || matches!(
                    node.role(),
                    StructureRole::Figure
                        | StructureRole::Formula
                        | StructureRole::Label
                        | StructureRole::Reference
                ))
    }) {
        if observed_fragments
            .get(&node.structure_node_id())
            .map_or(true, |fragments| {
                fragments.len() != 1 || !fragments.contains(&0)
            })
        {
            return Err(SelectedStructureBindingError::PaintOrder);
        }
    }
    let selected_annotations = select_annotations_v2(registry, pages, annotations)?;
    let mut receipt = SelectedStructureBindingReceiptV2 {
        structure_registry_sha256: registry.fingerprint(),
        authorization_sha256: authorization.fingerprint(),
        limits_sha256: limits.fingerprint(),
        selected_layout_sha256,
        selected_layout_fragment_count,
        pages: pages.to_vec(),
        paints: selected_paints,
        annotations: selected_annotations,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_selected_v2(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    Ok(receipt)
}

fn validate_selected_paint_binding_v2(
    registry: &StructureRegistryReceiptV2,
    node: &StructureNodeRecord,
    paint: &SelectedStructurePaintInputV2,
    preceding: Option<&SelectedStructurePaintV2>,
    vector_usage_ids: &mut BTreeSet<u32>,
    vector_structure_nodes: &mut BTreeSet<StructureNodeId>,
) -> Result<(), SelectedStructureBindingError> {
    match &paint.binding {
        SelectedStructurePaintBindingV2::Standard => {
            if node.vector_binding_v2().is_some() || node.equation_number_binding_v2().is_some() {
                return Err(SelectedStructureBindingError::InvalidVector);
            }
        }
        SelectedStructurePaintBindingV2::Vector(binding) => {
            let expected = node
                .vector_binding_v2()
                .ok_or(SelectedStructureBindingError::InvalidVector)?;
            if expected.kind() != binding.kind
                || expected.metrics_fingerprint() != binding.metrics_fingerprint
                || binding.display_command_fingerprint == [0; 32]
                || paint.semantic_fragment_ordinal != 0
                || !vector_usage_ids.insert(binding.usage_id)
                || !vector_structure_nodes.insert(node.structure_node_id())
                || match binding.kind {
                    PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::VectorFigure => {
                        node.role() != StructureRole::Figure
                    }
                    PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock => {
                        node.role() != StructureRole::Formula
                    }
                }
            {
                return Err(SelectedStructureBindingError::InvalidVector);
            }
        }
        SelectedStructurePaintBindingV2::EquationNumber(binding) => {
            let expected = node
                .equation_number_binding_v2()
                .ok_or(SelectedStructureBindingError::InvalidEquationNumber)?;
            let language = node
                .language_binding_v2()
                .ok_or(SelectedStructureBindingError::InvalidEquationNumber)?;
            let Some(parent_language) = language.parent_record_fingerprint() else {
                return Err(SelectedStructureBindingError::InvalidEquationNumber);
            };
            let preceding_matches = preceding.is_some_and(|previous| {
                previous.page_index == paint.page_index
                    && previous.paint_ordinal.checked_add(1) == Some(paint.paint_ordinal)
                    && matches!(
                        (&previous.binding, previous.owner),
                        (
                            SelectedStructurePaintBindingV2::Vector(vector),
                            SelectedStructurePaintOwner::Structure(parent_id)
                        ) if vector.kind == PrecomposedVectorKind::MathVectorBlock
                            && registry.node(parent_id).is_some_and(|parent| {
                                parent.owner() == StructureOwner::Source(binding.parent_owner)
                            })
                    )
            });
            if node.role() != StructureRole::Span
                || paint.semantic_fragment_ordinal != 0
                || expected.parent_owner() != binding.parent_owner
                || expected.text_span() != binding.text_span
                || expected.text_buffer_sha256() != binding.text_buffer_sha256
                || expected.exact_text() != binding.exact_text
                || expected.exact_text_sha256() != binding.exact_text_sha256
                || sha256(binding.exact_text.as_bytes()) != binding.exact_text_sha256
                || binding.shape_fingerprint == [0; 32]
                || binding.glyph_receipt_fingerprint == [0; 32]
                || binding.shape_language_fingerprint == [0; 32]
                || language.record_fingerprint() != binding.language_record_fingerprint
                || parent_language != binding.parent_language_record_fingerprint
                || !preceding_matches
            {
                return Err(SelectedStructureBindingError::InvalidEquationNumber);
            }
        }
    }
    Ok(())
}

fn select_annotations_v2(
    registry: &StructureRegistryReceiptV2,
    pages: &[SelectedStructurePage],
    annotations: &[SelectedStructureAnnotationInput],
) -> Result<Vec<SelectedStructureAnnotation>, SelectedStructureBindingError> {
    let link_nodes = registry
        .nodes()
        .iter()
        .filter(|node| node.role() == StructureRole::Link)
        .map(|node| match node.owner() {
            StructureOwner::Source(source) => Ok((source, node)),
            StructureOwner::Generated(_) => Err(SelectedStructureBindingError::InvalidAnnotation),
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut linked = BTreeSet::new();
    let mut next_annotation = BTreeMap::<u32, u32>::new();
    let mut previous_annotation = None;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(annotations.len())
        .map_err(|_| SelectedStructureBindingError::AllocationFailure)?;
    for (index, annotation) in annotations.iter().enumerate() {
        let node = link_nodes
            .get(&annotation.owner_node_id)
            .ok_or(SelectedStructureBindingError::InvalidAnnotation)?;
        let order_key = (annotation.page_index, annotation.annotation_ordinal);
        if usize::try_from(annotation.annotation_id) != Ok(index)
            || pages.get(annotation.page_index as usize).is_none()
            || *next_annotation.entry(annotation.page_index).or_insert(0)
                != annotation.annotation_ordinal
            || previous_annotation.is_some_and(|previous| previous >= order_key)
        {
            return Err(SelectedStructureBindingError::InvalidAnnotation);
        }
        previous_annotation = Some(order_key);
        *next_annotation
            .get_mut(&annotation.page_index)
            .ok_or(SelectedStructureBindingError::InvalidAnnotation)? += 1;
        linked.insert(annotation.owner_node_id);
        selected.push(SelectedStructureAnnotation {
            annotation_id: annotation.annotation_id,
            page_index: annotation.page_index,
            annotation_ordinal: annotation.annotation_ordinal,
            owner_node_id: annotation.owner_node_id,
            structure_node_id: node.structure_node_id(),
            accessible_name: node
                .accessible_name()
                .ok_or(SelectedStructureBindingError::InvalidAnnotation)?
                .to_owned(),
        });
    }
    if linked != link_nodes.keys().copied().collect() {
        return Err(SelectedStructureBindingError::InvalidAnnotation);
    }
    Ok(selected)
}

fn validate_selected_pages(
    pages: &[SelectedStructurePage],
    limits: &ValidatedResourceLimits,
) -> Result<(), SelectedStructureBindingError> {
    if pages.is_empty()
        || u32::try_from(pages.len()).map_or(true, |count| count > limits.get().max_pages)
        || pages.iter().enumerate().any(|(index, page)| {
            usize::try_from(page.page_index) != Ok(index)
                || page.width_raw <= 0
                || page.height_raw <= 0
        })
    {
        return Err(SelectedStructureBindingError::NonCanonicalPage);
    }
    Ok(())
}

fn encode_structure_role_vocabulary_v2() -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STRUCTURE_ROLE_VOCABULARY_ALGORITHM_V2);
    output.push_str(",\"roles\":[");
    for (index, role) in StructureRoleVocabularyV2.roles().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, role.pdf_name());
    }
    output.push_str("]}");
    output
}

fn encode_registry_v2(value: &StructureRegistryReceiptV2) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STRUCTURE_REGISTRY_ALGORITHM_V2);
    output.push_str(",\"authorization_sha256\":");
    push_hash(&mut output, value.authorization_sha256);
    output.push_str(",\"generated_node_count\":");
    output.push_str(&value.generated_node_count.to_string());
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"maximum_depth\":");
    output.push_str(&value.maximum_depth.to_string());
    output.push_str(",\"nodes\":[");
    for (index, node) in value.nodes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        encode_structure_node_v2(&mut output, node);
    }
    output.push_str("],\"package_sha256\":");
    push_hash(&mut output, value.package_sha256);
    output.push_str(",\"role_map\":{\"Em\":\"Span\",\"Exercise\":\"Div\",\"Proof\":\"Div\",\"Result\":\"Div\",\"Strong\":\"Span\"}");
    output.push_str(",\"role_vocabulary_sha256\":");
    push_hash(&mut output, value.role_vocabulary_sha256);
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, value.semantic_sha256);
    output.push_str(",\"semantics_sha256\":");
    push_hash(&mut output, value.semantics_sha256);
    output.push('}');
    output
}

fn encode_structure_node_v2(output: &mut String, node: &StructureNodeRecord) {
    output.push_str("{\"accessible_name\":");
    push_optional_string(output, node.accessible_name.as_deref());
    output.push_str(",\"actual_text\":");
    push_optional_string(output, node.actual_text.as_deref());
    output.push_str(",\"alternative\":");
    push_optional_string(output, node.alternative.as_deref());
    output.push_str(",\"children\":[");
    for (index, child) in node.children.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&child.get().to_string());
    }
    output.push_str("],\"equation_number_binding\":");
    if let Some(binding) = &node.equation_number_binding_v2 {
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
    } else {
        output.push_str("null");
    }
    output.push_str(",\"language\":");
    push_jcs_string(output, &node.language);
    output.push_str(",\"language_binding\":");
    if let Some(binding) = node.language_binding_v2 {
        output.push_str("{\"parent_record_fingerprint\":");
        if let Some(parent) = binding.parent_record_fingerprint() {
            push_hash(output, parent);
        } else {
            output.push_str("null");
        }
        output.push_str(",\"record_fingerprint\":");
        push_hash(output, binding.record_fingerprint());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"list_numbering\":");
    if let Some(numbering) = node.list_numbering {
        push_jcs_string(output, numbering.as_str());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"marker\":");
    push_optional_string(output, node.marker.as_deref());
    output.push_str(",\"outline_ids\":[");
    for (index, outline) in node.outline_ids.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&outline.to_string());
    }
    output.push_str("],\"owner\":");
    encode_owner(output, node.owner);
    output.push_str(",\"paint_required\":");
    output.push_str(if node.paint_required { "true" } else { "false" });
    output.push_str(",\"parent\":");
    if let Some(parent) = node.parent {
        output.push_str(&parent.get().to_string());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"related_nodes\":[");
    for (index, related) in node.related_nodes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&related.get().to_string());
    }
    output.push_str("],\"role\":");
    push_jcs_string(output, node.role.pdf_name());
    output.push_str(",\"source_span\":");
    if let Some(span) = node.source_span {
        push_source_span(output, span);
    } else {
        output.push_str("null");
    }
    output.push_str(",\"structure_id\":");
    push_optional_string(output, node.structure_id.as_deref());
    output.push_str(",\"structure_node_id\":");
    output.push_str(&node.structure_node_id.get().to_string());
    output.push_str(",\"table_attributes\":");
    if let Some(table) = &node.table_attributes {
        output.push_str("{\"colspan\":");
        output.push_str(&table.colspan.to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&table.column_ordinal.to_string());
        output.push_str(",\"header_ids\":[");
        for (index, header) in table.header_ids.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            push_jcs_string(output, header);
        }
        output.push_str("],\"row_ordinal\":");
        output.push_str(&table.row_ordinal.to_string());
        output.push_str(",\"rowspan\":");
        output.push_str(&table.rowspan.to_string());
        output.push_str(",\"section\":");
        push_jcs_string(output, table.section.as_str());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"vector_binding\":");
    if let Some(binding) = node.vector_binding_v2 {
        output.push_str("{\"kind\":");
        push_jcs_string(output, binding.kind().as_str());
        output.push_str(",\"metrics_fingerprint\":");
        push_hash(output, binding.metrics_fingerprint());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push('}');
}

fn encode_registry(value: &StructureRegistryReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STRUCTURE_REGISTRY_ALGORITHM);
    output.push_str(",\"authorization_sha256\":");
    push_hash(&mut output, value.authorization_sha256);
    output.push_str(",\"generated_node_count\":");
    output.push_str(&value.generated_node_count.to_string());
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"maximum_depth\":");
    output.push_str(&value.maximum_depth.to_string());
    output.push_str(",\"nodes\":[");
    for (index, node) in value.nodes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        encode_structure_node(&mut output, node);
    }
    output.push_str("],\"package_sha256\":");
    push_hash(&mut output, value.package_sha256);
    output.push_str(",\"role_map\":{\"Em\":\"Span\",\"Exercise\":\"Div\",\"Proof\":\"Div\",\"Result\":\"Div\",\"Strong\":\"Span\"}");
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, value.semantic_sha256);
    output.push_str(",\"semantics_sha256\":");
    push_hash(&mut output, value.semantics_sha256);
    output.push('}');
    output
}

fn encode_structure_node(output: &mut String, node: &StructureNodeRecord) {
    output.push_str("{\"accessible_name\":");
    push_optional_string(output, node.accessible_name.as_deref());
    output.push_str(",\"actual_text\":");
    push_optional_string(output, node.actual_text.as_deref());
    output.push_str(",\"alternative\":");
    push_optional_string(output, node.alternative.as_deref());
    output.push_str(",\"children\":[");
    for (index, child) in node.children.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&child.get().to_string());
    }
    output.push_str("],\"language\":");
    push_jcs_string(output, &node.language);
    output.push_str(",\"list_numbering\":");
    if let Some(numbering) = node.list_numbering {
        push_jcs_string(output, numbering.as_str());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"marker\":");
    push_optional_string(output, node.marker.as_deref());
    output.push_str(",\"outline_ids\":[");
    for (index, outline) in node.outline_ids.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&outline.to_string());
    }
    output.push_str("],\"owner\":");
    encode_owner(output, node.owner);
    output.push_str(",\"paint_required\":");
    output.push_str(if node.paint_required { "true" } else { "false" });
    output.push_str(",\"parent\":");
    if let Some(parent) = node.parent {
        output.push_str(&parent.get().to_string());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"related_nodes\":[");
    for (index, related) in node.related_nodes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&related.get().to_string());
    }
    output.push_str("],\"role\":");
    push_jcs_string(output, node.role.pdf_name());
    output.push_str(",\"source_span\":");
    if let Some(span) = node.source_span {
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
    output.push_str(",\"structure_id\":");
    push_optional_string(output, node.structure_id.as_deref());
    output.push_str(",\"structure_node_id\":");
    output.push_str(&node.structure_node_id.get().to_string());
    output.push_str(",\"table_attributes\":");
    if let Some(table) = &node.table_attributes {
        output.push_str("{\"colspan\":");
        output.push_str(&table.colspan.to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&table.column_ordinal.to_string());
        output.push_str(",\"header_ids\":[");
        for (index, header) in table.header_ids.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            push_jcs_string(output, header);
        }
        output.push_str("],\"row_ordinal\":");
        output.push_str(&table.row_ordinal.to_string());
        output.push_str(",\"rowspan\":");
        output.push_str(&table.rowspan.to_string());
        output.push_str(",\"section\":");
        push_jcs_string(output, table.section.as_str());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push('}');
}

fn encode_owner(output: &mut String, owner: StructureOwner) {
    match owner {
        StructureOwner::Source(node_id) => {
            output.push_str("{\"kind\":\"source\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push('}');
        }
        StructureOwner::Generated(key) => {
            output.push_str("{\"kind\":\"generated\",\"ordinal\":");
            output.push_str(&key.ordinal().to_string());
            output.push_str(",\"owner_node_id\":");
            output.push_str(&key.owner_node_id().get().to_string());
            output.push_str(",\"slot\":");
            push_jcs_string(output, key.slot().as_str());
            output.push('}');
        }
    }
}

fn encode_selected_v2(value: &SelectedStructureBindingReceiptV2) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, SELECTED_STRUCTURE_BINDING_ALGORITHM_V2);
    output.push_str(",\"annotations\":[");
    for (index, annotation) in value.annotations.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"accessible_name\":");
        push_jcs_string(&mut output, &annotation.accessible_name);
        output.push_str(",\"annotation_id\":");
        output.push_str(&annotation.annotation_id.to_string());
        output.push_str(",\"annotation_ordinal\":");
        output.push_str(&annotation.annotation_ordinal.to_string());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&annotation.owner_node_id.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&annotation.page_index.to_string());
        output.push_str(",\"structure_node_id\":");
        output.push_str(&annotation.structure_node_id.get().to_string());
        output.push('}');
    }
    output.push_str("],\"authorization_sha256\":");
    push_hash(&mut output, value.authorization_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"height\":");
        output.push_str(&page.height_raw.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"width\":");
        output.push_str(&page.width_raw.to_string());
        output.push('}');
    }
    output.push_str("],\"paints\":[");
    for (index, paint) in value.paints.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"actual_text\":");
        push_optional_string(&mut output, paint.actual_text.as_deref());
        output.push_str(",\"binding\":");
        encode_selected_paint_binding_v2(&mut output, &paint.binding);
        output.push_str(",\"language\":");
        push_optional_string(&mut output, paint.language.as_deref());
        output.push_str(",\"owner\":");
        match paint.owner {
            SelectedStructurePaintOwner::Structure(id) => {
                output.push_str("{\"kind\":\"structure\",\"structure_node_id\":");
                output.push_str(&id.get().to_string());
                output.push('}');
            }
            SelectedStructurePaintOwner::Artifact { class, occurrence } => {
                output.push_str("{\"class\":");
                push_jcs_string(&mut output, class.as_str());
                output.push_str(",\"kind\":\"artifact\",\"occurrence\":");
                output.push_str(&occurrence.to_string());
                output.push('}');
            }
        }
        output.push_str(",\"page_index\":");
        output.push_str(&paint.page_index.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&paint.paint_ordinal.to_string());
        output.push_str(",\"role\":");
        if let Some(role) = paint.role {
            push_jcs_string(&mut output, role.pdf_name());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"selected_paint_id\":");
        output.push_str(&paint.selected_paint_id.to_string());
        output.push_str(",\"semantic_fragment_ordinal\":");
        output.push_str(&paint.semantic_fragment_ordinal.to_string());
        output.push('}');
    }
    output.push_str("],\"selected_layout_fragment_count\":");
    output.push_str(&value.selected_layout_fragment_count.to_string());
    output.push_str(",\"selected_layout_sha256\":");
    push_hash(&mut output, value.selected_layout_sha256);
    output.push_str(",\"structure_registry_sha256\":");
    push_hash(&mut output, value.structure_registry_sha256);
    output.push('}');
    output
}

fn encode_selected_paint_binding_v2(
    output: &mut String,
    binding: &SelectedStructurePaintBindingV2,
) {
    match binding {
        SelectedStructurePaintBindingV2::Standard => {
            output.push_str("{\"kind\":\"standard\"}");
        }
        SelectedStructurePaintBindingV2::Vector(value) => {
            output.push_str("{\"display_command_fingerprint\":");
            push_hash(output, value.display_command_fingerprint);
            output.push_str(",\"kind\":\"vector\",\"metrics_fingerprint\":");
            push_hash(output, value.metrics_fingerprint);
            output.push_str(",\"usage_id\":");
            output.push_str(&value.usage_id.to_string());
            output.push_str(",\"vector_kind\":");
            push_jcs_string(output, value.kind.as_str());
            output.push('}');
        }
        SelectedStructurePaintBindingV2::EquationNumber(value) => {
            output.push_str("{\"exact_text\":");
            push_jcs_string(output, &value.exact_text);
            output.push_str(",\"exact_text_sha256\":");
            push_hash(output, value.exact_text_sha256);
            output.push_str(",\"glyph_receipt_fingerprint\":");
            push_hash(output, value.glyph_receipt_fingerprint);
            output.push_str(",\"kind\":\"equation_number\",\"language_record_fingerprint\":");
            push_hash(output, value.language_record_fingerprint);
            output.push_str(",\"parent_language_record_fingerprint\":");
            push_hash(output, value.parent_language_record_fingerprint);
            output.push_str(",\"parent_owner\":");
            output.push_str(&value.parent_owner.get().to_string());
            output.push_str(",\"shape_fingerprint\":");
            push_hash(output, value.shape_fingerprint);
            output.push_str(",\"shape_language_fingerprint\":");
            push_hash(output, value.shape_language_fingerprint);
            output.push_str(",\"text_buffer_sha256\":");
            push_hash(output, value.text_buffer_sha256);
            output.push_str(",\"text_span\":");
            push_text_span(output, value.text_span);
            output.push('}');
        }
    }
}

fn encode_selected(value: &SelectedStructureBindingReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, SELECTED_STRUCTURE_BINDING_ALGORITHM);
    output.push_str(",\"annotations\":[");
    for (index, annotation) in value.annotations.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"accessible_name\":");
        push_jcs_string(&mut output, &annotation.accessible_name);
        output.push_str(",\"annotation_id\":");
        output.push_str(&annotation.annotation_id.to_string());
        output.push_str(",\"annotation_ordinal\":");
        output.push_str(&annotation.annotation_ordinal.to_string());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&annotation.owner_node_id.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&annotation.page_index.to_string());
        output.push_str(",\"structure_node_id\":");
        output.push_str(&annotation.structure_node_id.get().to_string());
        output.push('}');
    }
    output.push_str("],\"authorization_sha256\":");
    push_hash(&mut output, value.authorization_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"height\":");
        output.push_str(&page.height_raw.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"width\":");
        output.push_str(&page.width_raw.to_string());
        output.push('}');
    }
    output.push_str("],\"paints\":[");
    for (index, paint) in value.paints.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"actual_text\":");
        push_optional_string(&mut output, paint.actual_text.as_deref());
        output.push_str(",\"language\":");
        push_optional_string(&mut output, paint.language.as_deref());
        output.push_str(",\"owner\":");
        match paint.owner {
            SelectedStructurePaintOwner::Structure(id) => {
                output.push_str("{\"kind\":\"structure\",\"structure_node_id\":");
                output.push_str(&id.get().to_string());
                output.push('}');
            }
            SelectedStructurePaintOwner::Artifact { class, occurrence } => {
                output.push_str("{\"class\":");
                push_jcs_string(&mut output, class.as_str());
                output.push_str(",\"kind\":\"artifact\",\"occurrence\":");
                output.push_str(&occurrence.to_string());
                output.push('}');
            }
        }
        output.push_str(",\"page_index\":");
        output.push_str(&paint.page_index.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&paint.paint_ordinal.to_string());
        output.push_str(",\"role\":");
        if let Some(role) = paint.role {
            push_jcs_string(&mut output, role.pdf_name());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"selected_paint_id\":");
        output.push_str(&paint.selected_paint_id.to_string());
        output.push_str(",\"semantic_fragment_ordinal\":");
        output.push_str(&paint.semantic_fragment_ordinal.to_string());
        output.push('}');
    }
    output.push_str("],\"selected_layout_fragment_count\":");
    output.push_str(&value.selected_layout_fragment_count.to_string());
    output.push_str(",\"selected_layout_sha256\":");
    push_hash(&mut output, value.selected_layout_sha256);
    output.push_str(",\"structure_registry_sha256\":");
    push_hash(&mut output, value.structure_registry_sha256);
    output.push('}');
    output
}

fn limits_fingerprint(limits: &ValidatedResourceLimits) -> [u8; 32] {
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
    let value = limits.get();
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

fn push_optional_string(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
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
    use typaxis_core::{M4ResourceLimits, ResourceLimits};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireStagingM4Block,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation_v2, validate_staging_structure_semantics_v2,
        StagingAccessibilityProfileAuthorizationV2, StagingAccessibilityProfileViewV2,
        StagingSemanticPackageParser,
    };

    const VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"
    ));

    #[test]
    fn vector_structure_registry_v2_maps_roles_alt_language_and_number() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(limits.base()),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, limits.base())
            .unwrap();
        let navigation = validate_staging_book_navigation_v2(&package, &limits).unwrap();
        let semantics =
            validate_staging_structure_semantics_v2(&package, &navigation, &limits).unwrap();
        assert!(semantics
            .canonical_jcs()
            .contains(STRUCTURE_REGISTRY_ALGORITHM_V2));
        let authorization = StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
            StagingAccessibilityProfileViewV2::new(&package, &navigation, &semantics, &limits)
                .unwrap(),
            sha256(b"layout-contract-tagged-profile-v2"),
            sha256(b"layout-contract-navigation-profile-v2"),
            &package,
            &navigation,
            &semantics,
            &limits,
        )
        .unwrap();
        let registry =
            build_structure_registry_v2(&package, &navigation, &semantics, &authorization, &limits)
                .unwrap();
        registry
            .verify(&package, &navigation, &semantics, &authorization, &limits)
            .unwrap();
        assert_eq!(StructureRoleVocabularyV2.roles().len(), 30);
        assert_eq!(
            registry.role_vocabulary_sha256(),
            StructureRoleVocabularyV2.fingerprint()
        );

        let expected = [
            (
                NodeId::new(3),
                PrecomposedVectorKind::InlineVector,
                StructureRole::Figure,
                "丸括弧で囲んだ二項目",
                None,
            ),
            (
                NodeId::new(4),
                PrecomposedVectorKind::MathVector,
                StructureRole::Formula,
                "xたすy",
                Some("xたすy"),
            ),
            (
                NodeId::new(5),
                PrecomposedVectorKind::VectorFigure,
                StructureRole::Figure,
                "配置図",
                None,
            ),
            (
                NodeId::new(6),
                PrecomposedVectorKind::MathVectorBlock,
                StructureRole::Formula,
                "xたすy、式1",
                Some("xたすy、式1"),
            ),
        ];
        for (owner, kind, role, alternative, actual_text) in expected {
            let node = registry.source_node(owner).unwrap();
            assert_eq!(node.role(), role);
            assert_eq!(node.alternative(), Some(alternative));
            assert_eq!(node.actual_text(), actual_text);
            assert_eq!(node.language(), "ja");
            assert!(node.language_binding_v2().is_some());
            assert_eq!(node.vector_binding_v2().unwrap().kind(), kind);
            assert!(node.paint_required());
        }
        let formula = registry.source_node(NodeId::new(6)).unwrap();
        let number = registry.source_node(NodeId::new(7)).unwrap();
        assert_eq!(number.role(), StructureRole::Span);
        assert_eq!(number.parent(), Some(formula.structure_node_id()));
        assert_eq!(formula.children().last(), Some(&number.structure_node_id()));
        let number_binding = number.equation_number_binding_v2().unwrap();
        assert_eq!(number_binding.parent_owner(), NodeId::new(6));
        assert_eq!(number_binding.exact_text(), "(1)");
        assert_eq!(
            sha256(number_binding.exact_text().as_bytes()),
            number_binding.exact_text_sha256()
        );

        let mut paints = Vec::new();
        for (index, owner) in [3, 4, 5, 6].into_iter().enumerate() {
            let node = registry.source_node(NodeId::new(owner)).unwrap();
            let vector = node.vector_binding_v2().unwrap();
            paints.push(SelectedStructurePaintInputV2 {
                selected_paint_id: index as u32,
                page_index: 0,
                paint_ordinal: index as u32,
                semantic_fragment_ordinal: 0,
                owner: SelectedStructurePaintOwner::Structure(node.structure_node_id()),
                binding: SelectedStructurePaintBindingV2::Vector(SelectedVectorPaintBindingV2 {
                    usage_id: index as u32,
                    kind: vector.kind(),
                    metrics_fingerprint: vector.metrics_fingerprint(),
                    display_command_fingerprint: sha256(
                        format!("layout-contract-vector-command-{index}").as_bytes(),
                    ),
                }),
            });
        }
        let number_language = number.language_binding_v2().unwrap();
        let parent_language = formula.language_binding_v2().unwrap();
        paints.push(SelectedStructurePaintInputV2 {
            selected_paint_id: 4,
            page_index: 0,
            paint_ordinal: 4,
            semantic_fragment_ordinal: 0,
            owner: SelectedStructurePaintOwner::Structure(number.structure_node_id()),
            binding: SelectedStructurePaintBindingV2::EquationNumber(
                SelectedEquationNumberPaintBindingV2 {
                    parent_owner: number_binding.parent_owner(),
                    text_span: number_binding.text_span(),
                    text_buffer_sha256: number_binding.text_buffer_sha256(),
                    exact_text: number_binding.exact_text().to_owned(),
                    exact_text_sha256: number_binding.exact_text_sha256(),
                    shape_fingerprint: sha256(b"equation-shape"),
                    glyph_receipt_fingerprint: sha256(b"equation-glyphs"),
                    shape_language_fingerprint: sha256(b"equation-language"),
                    language_record_fingerprint: number_language.record_fingerprint(),
                    parent_language_record_fingerprint: parent_language.record_fingerprint(),
                },
            ),
        });
        let pages = [SelectedStructurePage {
            page_index: 0,
            width_raw: 10_000_000,
            height_raw: 10_000_000,
        }];
        let selected = select_structure_bindings_v2(
            &registry,
            &authorization,
            &limits,
            sha256(b"layout-contract-selected-v2"),
            4,
            &pages,
            &paints,
            &[],
        )
        .unwrap();
        selected.verify(&registry, &authorization, &limits).unwrap();
        assert_eq!(selected.paints().len(), 5);
        assert_eq!(selected.paints()[1].actual_text(), Some("xたすy"));
        assert_eq!(selected.paints()[4].actual_text(), None);

        let mut duplicate_usage = paints.clone();
        let SelectedStructurePaintBindingV2::Vector(binding) = &mut duplicate_usage[1].binding
        else {
            unreachable!()
        };
        binding.usage_id = 0;
        let error = select_structure_bindings_v2(
            &registry,
            &authorization,
            &limits,
            sha256(b"layout-contract-selected-v2"),
            4,
            &pages,
            &duplicate_usage,
            &[],
        )
        .unwrap_err();
        assert_eq!(error, SelectedStructureBindingError::InvalidVector);
        assert!(error.to_string().starts_with("I9190:"));

        let mut swapped_number = paints.clone();
        swapped_number.swap(3, 4);
        for (index, paint) in swapped_number.iter_mut().enumerate() {
            paint.selected_paint_id = index as u32;
            paint.paint_ordinal = index as u32;
        }
        let error = select_structure_bindings_v2(
            &registry,
            &authorization,
            &limits,
            sha256(b"layout-contract-selected-v2"),
            4,
            &pages,
            &swapped_number,
            &[],
        )
        .unwrap_err();
        assert_eq!(error, SelectedStructureBindingError::InvalidEquationNumber);
        assert!(error.to_string().starts_with("I9190:"));

        let mut missing = paints.clone();
        missing.remove(0);
        for (index, paint) in missing.iter_mut().enumerate() {
            paint.selected_paint_id = index as u32;
            paint.paint_ordinal = index as u32;
        }
        let error = select_structure_bindings_v2(
            &registry,
            &authorization,
            &limits,
            sha256(b"layout-contract-selected-v2"),
            4,
            &pages,
            &missing,
            &[],
        )
        .unwrap_err();
        assert_eq!(error, SelectedStructureBindingError::MissingPaint);

        let mut wrong_metrics = paints.clone();
        let SelectedStructurePaintBindingV2::Vector(binding) = &mut wrong_metrics[0].binding else {
            unreachable!()
        };
        binding.metrics_fingerprint = sha256(b"wrong-vector-metrics");
        assert_eq!(
            select_structure_bindings_v2(
                &registry,
                &authorization,
                &limits,
                sha256(b"layout-contract-selected-v2"),
                4,
                &pages,
                &wrong_metrics,
                &[],
            ),
            Err(SelectedStructureBindingError::InvalidVector)
        );

        let mut extra_vector = paints.clone();
        let mut duplicate = extra_vector[3].clone();
        duplicate.selected_paint_id = 4;
        duplicate.paint_ordinal = 4;
        let SelectedStructurePaintBindingV2::Vector(binding) = &mut duplicate.binding else {
            unreachable!()
        };
        binding.usage_id = 4;
        extra_vector.insert(4, duplicate);
        extra_vector[5].selected_paint_id = 5;
        extra_vector[5].paint_ordinal = 5;
        assert_eq!(
            select_structure_bindings_v2(
                &registry,
                &authorization,
                &limits,
                sha256(b"layout-contract-selected-v2"),
                4,
                &pages,
                &extra_vector,
                &[],
            ),
            Err(SelectedStructureBindingError::InvalidVector)
        );

        let mutations: [fn(&mut StructureNodeRecord); 4] = [
            |node: &mut StructureNodeRecord| node.role = StructureRole::Span,
            |node: &mut StructureNodeRecord| node.alternative = Some("wrong alt".to_owned()),
            |node: &mut StructureNodeRecord| node.language = "en-US".to_owned(),
            |node: &mut StructureNodeRecord| node.parent = None,
        ];
        for mutate in mutations {
            let mut tampered = registry.clone();
            let id = tampered
                .source_node(NodeId::new(4))
                .unwrap()
                .structure_node_id();
            mutate(&mut tampered.nodes[id.get() as usize]);
            let error = tampered
                .verify(&package, &navigation, &semantics, &authorization, &limits)
                .unwrap_err();
            assert_eq!(error, StructureRegistryError::ReceiptMismatch);
            assert!(error.to_string().starts_with("I9190:"));
        }
    }

    #[test]
    fn vector_structure_registry_v2_omits_equation_child_for_null_number() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(limits.base()),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
            unreachable!()
        };
        let WireStagingM4Block::MathVectorBlock {
            equation_number, ..
        } = &mut blocks[2]
        else {
            unreachable!()
        };
        *equation_number = None;
        wire.replace_typed_regions(document, wire.resources().clone());
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                encoded.as_bytes(),
                &DocumentPackageDecodePolicy::new(limits.base()),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, limits.base())
            .unwrap();
        let navigation = validate_staging_book_navigation_v2(&package, &limits).unwrap();
        let semantics =
            validate_staging_structure_semantics_v2(&package, &navigation, &limits).unwrap();
        let authorization = StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
            StagingAccessibilityProfileViewV2::new(&package, &navigation, &semantics, &limits)
                .unwrap(),
            sha256(b"layout-contract-tagged-profile-v2-null-number"),
            sha256(b"layout-contract-navigation-profile-v2-null-number"),
            &package,
            &navigation,
            &semantics,
            &limits,
        )
        .unwrap();
        let registry =
            build_structure_registry_v2(&package, &navigation, &semantics, &authorization, &limits)
                .unwrap();
        let formula = registry.source_node(NodeId::new(6)).unwrap();
        assert_eq!(formula.role(), StructureRole::Formula);
        assert!(formula.children().is_empty());
        assert!(registry.source_node(NodeId::new(7)).is_none());
        assert!(registry
            .nodes()
            .iter()
            .all(|node| node.equation_number_binding_v2().is_none()));
    }
}
