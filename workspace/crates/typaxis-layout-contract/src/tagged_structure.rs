use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{push_jcs_string, sha256, NodeId, SourceSpan, ValidatedResourceLimits};
use typaxis_syntax::{
    StagingAccessibilityProfileAuthorization, StagingStructureSemanticKind,
    StagingStructureSemanticRecord, StagingStructureTableSection, ValidatedStagingBookNavigation,
    ValidatedStagingSemanticPackage, ValidatedStagingStructureSemantics,
};

pub const STRUCTURE_REGISTRY_ALGORITHM: &str = "typaxis.structure-registry/1";
pub const SELECTED_STRUCTURE_BINDING_ALGORITHM: &str = "typaxis.selected-structure-binding/1";

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureNodeRecord {
    structure_node_id: StructureNodeId,
    owner: StructureOwner,
    source_span: Option<SourceSpan>,
    role: StructureRole,
    parent: Option<StructureNodeId>,
    children: Vec<StructureNodeId>,
    language: String,
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

struct RegistryBuilder<'a> {
    semantics: &'a ValidatedStagingStructureSemantics,
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
        semantics,
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
        let children = match record.kind() {
            StagingStructureSemanticKind::ListItem { marker } => {
                let label = self.allocate_generated(
                    source,
                    GeneratedStructureSlot::ListLabel,
                    StructureRole::Label,
                    id,
                    record.language(),
                    true,
                    None,
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
            StagingStructureSemanticKind::Figure { has_caption, .. } if *has_caption => {
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
                    None,
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
                    None,
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
        self.allocate(
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
        )
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
        StagingStructureSemanticKind::FootnoteDefinition { .. } => {
            (StructureRole::Note, None, None, false, None, None)
        }
        StagingStructureSemanticKind::Text { .. } => {
            (StructureRole::Span, None, None, true, None, None)
        }
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
            None,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedStructureBindingError {
    RegistryMismatch,
    NonCanonicalPage,
    MissingPaint,
    ExtraPaint,
    PaintOrder,
    InvalidArtifact,
    InvalidAnnotation,
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

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}
