use super::*;
use typaxis_document::{
    FontMediaDeclaration, FontMediaType, ImageMediaDeclaration, ImageMediaType,
    SemanticContainerKind, StagingM4Block, StagingM4BlockCommon, StagingM4Document,
    StagingM4FontFaceDeclaration, StagingM4FootnoteDefinition, StagingM4ImageDeclaration,
    StagingM4ListItem, StagingM4ResourceCatalog, StagingM4TableCell, StagingM4TableRow,
};
use typaxis_document_package::{
    DecodedStagingSemanticDocumentPackage, WireFontMediaType, WireImageMediaType,
    WireStagingM4Block, WireStagingM4Document, WireStagingM4DocumentPackage, WireStagingM4Inline,
    WireStagingM4LinkTarget, WireStagingM4ResourceCatalog, WireStagingM4Source,
    WireStagingM4TextBuffer, WireStagingSourceSpan, WireStagingStyleSheet, WireStagingStyleValue,
    WireStagingTextSpan,
};
use typaxis_style::{
    cascade_staging_semantic_container_style, cascade_staging_semantic_descendant_style,
    SemanticContainerComputedStyle, SemanticContainerInheritanceStyle, SemanticContainerStyleKind,
};

const SEMANTIC_SYNTAX_FINGERPRINT_ALGORITHM: &str = "typaxis.semantic-container-syntax/1";
const STAGING_PROFILE_ID: &str = "typaxis.machine-pdf/production-book-1";
const STAGING_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.production-book-profile-receipt/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticSyntaxError {
    InvalidNodeOrder,
    InvalidSource,
    InvalidSourceSpan,
    InvalidClass,
    InvalidNesting,
    EmptyContainer(NodeId),
    InvalidBlock(NodeId),
    InvalidInline,
    InvalidResource,
    InvalidStyle,
    InapplicableStyle,
    AstNodeLimit,
    AstDepthLimit,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSemanticSyntaxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidNodeOrder => {
                formatter.write_str("P1102: semantic NodeIds are not dense preorder")
            }
            Self::InvalidSource => formatter.write_str("P1102: invalid semantic source catalog"),
            Self::InvalidSourceSpan => {
                formatter.write_str("P1102: semantic source span ownership mismatch")
            }
            Self::InvalidClass => {
                formatter.write_str("P1102: semantic class list is not canonical")
            }
            Self::InvalidNesting => {
                formatter.write_str("L5100: semantic_container is not allowed in this owner")
            }
            Self::EmptyContainer(owner) => write!(
                formatter,
                "L5100: recursively empty semantic_container at node {}",
                owner.get()
            ),
            Self::InvalidBlock(owner) => write!(
                formatter,
                "L5100: invalid block owned by node {}",
                owner.get()
            ),
            Self::InvalidInline => formatter.write_str("L5100: invalid semantic inline nesting"),
            Self::InvalidResource => formatter.write_str("P1102: invalid declared-media resource"),
            Self::InvalidStyle => formatter.write_str("L5101: invalid semantic_container style"),
            Self::InapplicableStyle => {
                formatter.write_str("L5101: inapplicable semantic_container property")
            }
            Self::AstNodeLimit => formatter.write_str("P1102: semantic AST exceeds max_ast_nodes"),
            Self::AstDepthLimit => {
                formatter.write_str("P1102: semantic AST exceeds max_ast_nesting_depth")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: semantic syntax receipt mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("P1102: semantic syntax allocation failed")
            }
        }
    }
}

impl std::error::Error for StagingSemanticSyntaxError {}

/// Syntax-owned proof of the complete contract-1.4 semantic and declared-media
/// lowering. The original typed carrier is retained for a checked canonical
/// re-encode; no public contract decoder can consume it.
#[derive(Debug)]
pub struct ValidatedStagingSemanticPackage {
    wire: WireStagingM4DocumentPackage,
    limits: ValidatedResourceLimits,
    document: StagingM4Document,
    resources: StagingM4ResourceCatalog,
    computed_styles: BTreeMap<NodeId, SemanticContainerComputedStyle>,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    semantic_jcs: String,
}

/// Dependency-inversion view of the profile-owned authorization consumed by
/// downstream staging phases. Its private fields prevent callers from
/// implementing a look-alike receipt; construction rechecks the fixed M4
/// domain and exact effective limits before producing the profile fingerprint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerProfileView {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    limits: ValidatedResourceLimits,
    container_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSemanticContainerProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        package.checked_wire()?;
        if package.limits() != limits {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let mut container_count = 0u32;
        validate_profile_container_domain(&package.document.blocks, &mut container_count)?;
        for footnote in &package.document.footnotes {
            validate_profile_container_domain(&footnote.blocks, &mut container_count)?;
        }
        if usize::try_from(container_count) != Ok(package.semantic_container_count()) {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let canonical_jcs = encode_profile_view(package, limits, container_count);
        Ok(Self {
            package_sha256: package.canonical_jcs_sha256(),
            semantic_fingerprint: package.semantic_fingerprint(),
            limits: limits.clone(),
            container_count,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn semantic_fingerprint(&self) -> [u8; 32] {
        self.semantic_fingerprint
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }

    pub const fn container_count(&self) -> u32 {
        self.container_count
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn validate_profile_container_domain(
    blocks: &[StagingM4Block],
    count: &mut u32,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer { common, blocks, .. } => {
                if !blocks.iter().any(StagingM4Block::is_semantically_nonempty) {
                    return Err(StagingSemanticSyntaxError::EmptyContainer(common.node_id));
                }
                *count = count
                    .checked_add(1)
                    .ok_or(StagingSemanticSyntaxError::AstNodeLimit)?;
                validate_profile_container_domain(blocks, count)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    validate_profile_container_domain(&item.blocks, count)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    validate_profile_container_domain(&cell.blocks, count)?;
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                validate_profile_container_domain(caption, count)?;
            }
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. } => {}
        }
    }
    Ok(())
}

fn encode_profile_view(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    container_count: u32,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_PROFILE_RECEIPT_ALGORITHM);
    output.push_str(",\"canonical_package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"container_count\":");
    output.push_str(&container_count.to_string());
    output.push_str(",\"contract\":\"typaxis.contract/1.4\"");
    output.push_str(",\"effective_limits\":{");
    push_profile_limits(&mut output, limits);
    output.push('}');
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, STAGING_PROFILE_ID);
    output.push_str(",\"semantic_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push('}');
    output
}

fn push_profile_limits(output: &mut String, limits: &ValidatedResourceLimits) {
    let limits = limits.get();
    macro_rules! fields {
        ($(($name:literal, $value:expr)),+ $(,)?) => {{
            let mut first = true;
            $(
                if !first {
                    output.push(',');
                }
                first = false;
                output.push_str(concat!("\"", $name, "\":"));
                output.push_str(&$value.to_string());
            )+
            let _ = first;
        }};
    }
    fields!(
        ("max_ast_nesting_depth", limits.max_ast_nesting_depth),
        ("max_ast_nodes", limits.max_ast_nodes),
        ("max_cids_per_font", limits.max_cids_per_font),
        (
            "max_column_balance_candidates",
            limits.max_column_balance_candidates
        ),
        ("max_decoded_image_bytes", limits.max_decoded_image_bytes),
        (
            "max_document_package_bytes",
            limits.max_document_package_bytes
        ),
        ("max_float_carry_pages", limits.max_float_carry_pages),
        ("max_float_queue", limits.max_float_queue),
        ("max_font_bytes", limits.max_font_bytes),
        ("max_fonts", limits.max_fonts),
        (
            "max_footnote_reflows_per_page",
            limits.max_footnote_reflows_per_page
        ),
        ("max_fragments", limits.max_fragments),
        ("max_image_bytes", limits.max_image_bytes),
        ("max_image_pixels", limits.max_image_pixels),
        ("max_images", limits.max_images),
        ("max_include_depth", limits.max_include_depth),
        ("max_include_files", limits.max_include_files),
        ("max_input_bytes", limits.max_input_bytes),
        ("max_json_nesting_depth", limits.max_json_nesting_depth),
        ("max_layout_passes", limits.max_layout_passes),
        ("max_line_reshape_passes", limits.max_line_reshape_passes),
        ("max_output_bytes", limits.max_output_bytes),
        ("max_page_break_lookback", limits.max_page_break_lookback),
        ("max_pages", limits.max_pages),
        ("max_pdf_objects", limits.max_pdf_objects),
        ("max_resource_bytes", limits.max_resource_bytes),
        (
            "max_shaping_context_bytes",
            limits.max_shaping_context_bytes
        ),
        ("max_source_bytes", limits.max_source_bytes),
        ("max_spool_bytes", limits.max_spool_bytes),
        ("max_style_rules", limits.max_style_rules),
        ("max_text_buffer_bytes", limits.max_text_buffer_bytes),
        ("max_text_bytes", limits.max_text_bytes),
        ("max_uri_bytes", limits.max_uri_bytes),
    );
}

impl ValidatedStagingSemanticPackage {
    pub const fn document(&self) -> &StagingM4Document {
        &self.document
    }
    pub const fn resources(&self) -> &StagingM4ResourceCatalog {
        &self.resources
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }
    pub const fn canonical_jcs_sha256(&self) -> [u8; 32] {
        self.canonical_jcs_sha256
    }
    pub const fn semantic_fingerprint(&self) -> [u8; 32] {
        self.semantic_fingerprint
    }
    pub fn semantic_jcs(&self) -> &str {
        &self.semantic_jcs
    }
    pub fn computed_style(&self, owner: NodeId) -> Option<&SemanticContainerComputedStyle> {
        self.computed_styles.get(&owner)
    }
    pub fn semantic_container_count(&self) -> usize {
        self.computed_styles.len()
    }
    pub fn checked_wire(
        &self,
    ) -> Result<&WireStagingM4DocumentPackage, StagingSemanticSyntaxError> {
        let observed = encode_semantic_receipt(
            &self.document,
            &self.resources,
            &self.computed_styles,
            self.canonical_jcs_sha256,
        );
        if observed != self.semantic_jcs || sha256(observed.as_bytes()) != self.semantic_fingerprint
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(&self.wire)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingSemanticPackageParser;

impl StagingSemanticPackageParser {
    pub const fn new() -> Self {
        Self
    }

    pub fn parse(
        &self,
        decoded: DecodedStagingSemanticDocumentPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<ValidatedStagingSemanticPackage, StagingSemanticSyntaxError> {
        if decoded.limits() != limits {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let raw_sha256 = decoded.raw_sha256();
        let canonical_jcs_sha256 = decoded.canonical_jcs_sha256();
        let wire = decoded.into_wire();
        let sources = parse_source_lengths(wire.sources())?;
        let text_buffers = parse_text_buffers(wire.text_buffers())?;
        let mut validator = SemanticValidator {
            sources: &sources,
            text_buffers: &text_buffers,
            next_node_id: 0,
            node_count: 0,
            limits,
        };
        validator.node(wire.document().node_id, None, 1)?;
        let document = lower_document(wire.document(), &mut validator)?;
        let resources = lower_resources(wire.resources())?;
        let rules = lower_semantic_style_rules(wire.style_sheet(), limits)?;
        let mut computed_styles = BTreeMap::new();
        collect_computed_styles(&document.blocks, &rules, None, &mut computed_styles)?;
        for footnote in &document.footnotes {
            collect_computed_styles(&footnote.blocks, &rules, None, &mut computed_styles)?;
        }
        if computed_styles.is_empty() {
            return Err(StagingSemanticSyntaxError::InvalidNesting);
        }
        let semantic_jcs = encode_semantic_receipt(
            &document,
            &resources,
            &computed_styles,
            canonical_jcs_sha256,
        );
        Ok(ValidatedStagingSemanticPackage {
            wire,
            limits: limits.clone(),
            document,
            resources,
            computed_styles,
            raw_sha256,
            canonical_jcs_sha256,
            semantic_fingerprint: sha256(semantic_jcs.as_bytes()),
            semantic_jcs,
        })
    }
}

struct SemanticValidator<'a> {
    sources: &'a BTreeMap<u32, u32>,
    text_buffers: &'a BTreeMap<u32, String>,
    next_node_id: u32,
    node_count: u64,
    limits: &'a ValidatedResourceLimits,
}

fn parse_text_buffers(
    buffers: &[WireStagingM4TextBuffer],
) -> Result<BTreeMap<u32, String>, StagingSemanticSyntaxError> {
    let mut result = BTreeMap::new();
    for (index, buffer) in buffers.iter().enumerate() {
        if usize::try_from(buffer.text_id) != Ok(index)
            || result.insert(buffer.text_id, buffer.utf8.clone()).is_some()
        {
            return Err(StagingSemanticSyntaxError::InvalidInline);
        }
    }
    Ok(result)
}

impl SemanticValidator<'_> {
    fn node(
        &mut self,
        node_id: u32,
        span: Option<WireStagingSourceSpan>,
        depth: u32,
    ) -> Result<(), StagingSemanticSyntaxError> {
        if node_id != self.next_node_id {
            return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
        }
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .ok_or(StagingSemanticSyntaxError::AstNodeLimit)?;
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or(StagingSemanticSyntaxError::AstNodeLimit)?;
        if self.node_count > self.limits.get().max_ast_nodes {
            return Err(StagingSemanticSyntaxError::AstNodeLimit);
        }
        if depth > self.limits.get().max_ast_nesting_depth {
            return Err(StagingSemanticSyntaxError::AstDepthLimit);
        }
        if let Some(span) = span {
            self.validate_span(span)?;
        }
        Ok(())
    }

    fn validate_span(&self, span: WireStagingSourceSpan) -> Result<(), StagingSemanticSyntaxError> {
        let length = self
            .sources
            .get(&span.source_id)
            .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)?;
        if span.start_byte > span.end_byte || span.end_byte > *length {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        Ok(())
    }
}

fn parse_source_lengths(
    sources: &[WireStagingM4Source],
) -> Result<BTreeMap<u32, u32>, StagingSemanticSyntaxError> {
    let mut result = BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        if usize::try_from(source.source_id) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidSource);
        }
        if result
            .insert(source.source_id, source.utf8_byte_length)
            .is_some()
        {
            return Err(StagingSemanticSyntaxError::InvalidSource);
        }
    }
    Ok(result)
}

fn lower_document(
    wire: &WireStagingM4Document,
    validator: &mut SemanticValidator<'_>,
) -> Result<StagingM4Document, StagingSemanticSyntaxError> {
    let blocks = lower_blocks(&wire.blocks, validator, None, 2)?;
    let mut footnotes = Vec::new();
    footnotes
        .try_reserve_exact(wire.footnotes.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for footnote in &wire.footnotes {
        validator.node(footnote.node_id, Some(footnote.span), 2)?;
        let span = lower_span(footnote.span)?;
        footnotes.push(StagingM4FootnoteDefinition {
            node_id: NodeId::new(footnote.node_id),
            span,
            blocks: lower_blocks(&footnote.blocks, validator, Some(footnote.span), 3)?,
        });
    }
    Ok(StagingM4Document {
        node_id: NodeId::new(wire.node_id),
        blocks,
        footnotes,
    })
}

fn lower_blocks(
    values: &[WireStagingM4Block],
    validator: &mut SemanticValidator<'_>,
    semantic_owner: Option<WireStagingSourceSpan>,
    depth: u32,
) -> Result<Vec<StagingM4Block>, StagingSemanticSyntaxError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(values.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    let mut previous_direct_start = None;
    for block in values {
        let span = wire_block_span(block);
        validator.node(block.node_id(), Some(span), depth)?;
        validate_classes(block.classes())?;
        if let Some(owner) = semantic_owner {
            validate_owned_span(owner, span)?;
            if previous_direct_start.is_some_and(|previous| previous > span.start_byte) {
                return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
            }
            previous_direct_start = Some(span.start_byte);
        }
        let common = StagingM4BlockCommon {
            node_id: NodeId::new(block.node_id()),
            span: lower_span(span)?,
            classes: block.classes().to_vec(),
        };
        let lowered = match block {
            WireStagingM4Block::Paragraph { children, .. } => {
                let has_authored_content =
                    validate_inlines(children, validator, Some(span), depth + 1)?;
                StagingM4Block::Paragraph {
                    common,
                    has_authored_content,
                }
            }
            WireStagingM4Block::Heading {
                level, children, ..
            } => {
                if !(1..=6).contains(level) {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                let has_authored_content =
                    validate_inlines(children, validator, Some(span), depth + 1)?;
                StagingM4Block::Heading {
                    common,
                    has_authored_content,
                }
            }
            WireStagingM4Block::List {
                items,
                ordered,
                start,
                ..
            } => {
                if items.is_empty()
                    || (*ordered && start.map_or(true, |value| value == 0))
                    || (!*ordered && start.is_some())
                {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                let mut lowered_items = Vec::new();
                let mut previous_item_start = None;
                for item in items {
                    validator.node(item.node_id, Some(item.span), depth + 1)?;
                    validate_owned_span(span, item.span)?;
                    if previous_item_start.is_some_and(|previous| previous > item.span.start_byte) {
                        return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
                    }
                    previous_item_start = Some(item.span.start_byte);
                    lowered_items.push(StagingM4ListItem {
                        node_id: NodeId::new(item.node_id),
                        span: lower_span(item.span)?,
                        blocks: lower_blocks(&item.blocks, validator, Some(item.span), depth + 2)?,
                    });
                }
                StagingM4Block::List {
                    common,
                    items: lowered_items,
                }
            }
            WireStagingM4Block::Table {
                columns,
                head,
                body,
                ..
            } => {
                if columns.is_empty() || (head.is_empty() && body.is_empty()) {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                StagingM4Block::Table {
                    common,
                    head: lower_rows(head, validator, span, depth + 1)?,
                    body: lower_rows(body, validator, span, depth + 1)?,
                }
            }
            WireStagingM4Block::Figure {
                placement,
                alt,
                caption,
                ..
            } => {
                if !matches!(placement.as_str(), "block" | "float") {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                StagingM4Block::Figure {
                    common,
                    has_nonempty_alternative: !alt.is_empty(),
                    caption: lower_blocks(caption, validator, Some(span), depth + 1)?,
                }
            }
            WireStagingM4Block::PageBreak { .. } => StagingM4Block::PageBreak { common },
            WireStagingM4Block::SemanticContainer {
                semantic_kind,
                blocks,
                ..
            } => {
                if blocks.is_empty() {
                    return Err(StagingSemanticSyntaxError::EmptyContainer(common.node_id));
                }
                let semantic_kind = match semantic_kind {
                    typaxis_document_package::WireStagingSemanticContainerKind::Result => {
                        SemanticContainerKind::Result
                    }
                    typaxis_document_package::WireStagingSemanticContainerKind::Proof => {
                        SemanticContainerKind::Proof
                    }
                    typaxis_document_package::WireStagingSemanticContainerKind::Exercise => {
                        SemanticContainerKind::Exercise
                    }
                };
                let blocks = lower_blocks(blocks, validator, Some(span), depth + 1)?;
                StagingM4Block::SemanticContainer {
                    common,
                    semantic_kind,
                    blocks,
                }
            }
        };
        output.push(lowered);
    }
    Ok(output)
}

fn lower_rows(
    rows: &[typaxis_document_package::WireStagingM4TableRow],
    validator: &mut SemanticValidator<'_>,
    table_owner: WireStagingSourceSpan,
    depth: u32,
) -> Result<Vec<StagingM4TableRow>, StagingSemanticSyntaxError> {
    let mut output = Vec::new();
    let mut previous_row_start = None;
    for row in rows {
        validator.node(row.node_id, Some(row.span), depth)?;
        validate_owned_span(table_owner, row.span)?;
        if previous_row_start.is_some_and(|previous| previous > row.span.start_byte) {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        previous_row_start = Some(row.span.start_byte);
        let mut cells = Vec::new();
        let mut previous_cell_start = None;
        for cell in &row.cells {
            validator.node(cell.node_id, Some(cell.span), depth + 1)?;
            validate_owned_span(row.span, cell.span)?;
            if previous_cell_start.is_some_and(|previous| previous > cell.span.start_byte) {
                return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
            }
            previous_cell_start = Some(cell.span.start_byte);
            cells.push(StagingM4TableCell {
                node_id: NodeId::new(cell.node_id),
                span: lower_span(cell.span)?,
                colspan: NonZeroU16::new(cell.colspan).ok_or(
                    StagingSemanticSyntaxError::InvalidBlock(NodeId::new(cell.node_id)),
                )?,
                rowspan: NonZeroU16::new(cell.rowspan).ok_or(
                    StagingSemanticSyntaxError::InvalidBlock(NodeId::new(cell.node_id)),
                )?,
                blocks: lower_blocks(&cell.blocks, validator, Some(cell.span), depth + 2)?,
            });
        }
        output.push(StagingM4TableRow {
            node_id: NodeId::new(row.node_id),
            span: lower_span(row.span)?,
            cells,
        });
    }
    Ok(output)
}

fn validate_inlines(
    values: &[WireStagingM4Inline],
    validator: &mut SemanticValidator<'_>,
    owner: Option<WireStagingSourceSpan>,
    depth: u32,
) -> Result<bool, StagingSemanticSyntaxError> {
    let mut has_authored_content = false;
    let mut previous_start = None;
    for value in values {
        let span = value.span();
        validator.node(value.node_id(), Some(span), depth)?;
        if let Some(owner) = owner {
            validate_owned_span(owner, span)?;
        }
        if previous_start.is_some_and(|previous| previous > span.start_byte) {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        previous_start = Some(span.start_byte);
        let inline_has_content = match value {
            WireStagingM4Inline::Text { text_span, .. } => {
                validate_text_span(*text_span, validator)?
            }
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. } => {
                validate_inlines(children, validator, Some(span), depth + 1)?
            }
            WireStagingM4Inline::Link {
                target, children, ..
            } => {
                let valid_target = match target {
                    WireStagingM4LinkTarget::Internal { anchor_id } => {
                        AnchorId::new(anchor_id.clone()).is_ok()
                    }
                    WireStagingM4LinkTarget::Uri { uri } => SafeUri::new(uri.clone()).is_ok(),
                };
                if !valid_target {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                validate_inlines(children, validator, Some(span), depth + 1)?
            }
            WireStagingM4Inline::Anchor { anchor_id, .. } => {
                if AnchorId::new(anchor_id.clone()).is_err() {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                false
            }
            WireStagingM4Inline::Reference { target, .. } => {
                if AnchorId::new(target.clone()).is_err() {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                true
            }
            WireStagingM4Inline::FootnoteReference { footnote_id, .. } => {
                if FootnoteId::new(footnote_id.clone()).is_err() {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                true
            }
            WireStagingM4Inline::SoftBreak { .. } | WireStagingM4Inline::HardBreak { .. } => false,
        };
        has_authored_content |= inline_has_content;
    }
    Ok(has_authored_content)
}

fn validate_text_span(
    value: WireStagingTextSpan,
    validator: &SemanticValidator<'_>,
) -> Result<bool, StagingSemanticSyntaxError> {
    let text = validator
        .text_buffers
        .get(&value.text_id)
        .ok_or(StagingSemanticSyntaxError::InvalidInline)?;
    let start_index =
        usize::try_from(value.start_byte).map_err(|_| StagingSemanticSyntaxError::InvalidInline)?;
    let end_index =
        usize::try_from(value.end_byte).map_err(|_| StagingSemanticSyntaxError::InvalidInline)?;
    if value.start_byte > value.end_byte
        || end_index > text.len()
        || !text.is_char_boundary(start_index)
        || !text.is_char_boundary(end_index)
    {
        return Err(StagingSemanticSyntaxError::InvalidInline);
    }
    Ok(value.start_byte < value.end_byte)
}

fn wire_block_span(block: &WireStagingM4Block) -> WireStagingSourceSpan {
    match block {
        WireStagingM4Block::Paragraph { span, .. }
        | WireStagingM4Block::Heading { span, .. }
        | WireStagingM4Block::List { span, .. }
        | WireStagingM4Block::Table { span, .. }
        | WireStagingM4Block::Figure { span, .. }
        | WireStagingM4Block::PageBreak { span, .. }
        | WireStagingM4Block::SemanticContainer { span, .. } => *span,
    }
}

fn validate_owned_span(
    owner: WireStagingSourceSpan,
    child: WireStagingSourceSpan,
) -> Result<(), StagingSemanticSyntaxError> {
    if owner.source_id != child.source_id
        || child.start_byte < owner.start_byte
        || child.end_byte > owner.end_byte
    {
        return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
    }
    Ok(())
}

fn lower_span(span: WireStagingSourceSpan) -> Result<SourceSpan, StagingSemanticSyntaxError> {
    SourceSpan::new(
        SourceId::new(span.source_id),
        Utf8ByteOffset::new(span.start_byte),
        Utf8ByteOffset::new(span.end_byte),
    )
    .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)
}

fn validate_classes(classes: &[String]) -> Result<(), StagingSemanticSyntaxError> {
    let mut previous: Option<&[u8]> = None;
    for class in classes {
        if !is_style_identifier(class) || previous.is_some_and(|value| value >= class.as_bytes()) {
            return Err(StagingSemanticSyntaxError::InvalidClass);
        }
        previous = Some(class.as_bytes());
    }
    Ok(())
}

fn lower_resources(
    wire: &WireStagingM4ResourceCatalog,
) -> Result<StagingM4ResourceCatalog, StagingSemanticSyntaxError> {
    let mut font_faces = Vec::new();
    let mut families = BTreeSet::new();
    font_faces
        .try_reserve_exact(wire.font_faces.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for (index, font) in wire.font_faces.iter().enumerate() {
        if usize::try_from(font.font_face_id) != Ok(index)
            || font.family.trim().is_empty()
            || font.family.chars().any(char::is_control)
            || !families.insert(font.family.as_str())
            || (font.media_type == WireFontMediaType::SfntTrueTypeGlyf && font.face_index != 0)
        {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        font_faces.push(StagingM4FontFaceDeclaration {
            font_face_id: FontFaceId::new(font.font_face_id),
            family: font.family.clone(),
            uri: PortablePath::new(font.uri.clone())
                .map_err(|_| StagingSemanticSyntaxError::InvalidResource)?,
            face_index: font.face_index,
            expected_sha256: parse_optional_hash(font.expected_sha256.as_deref())?,
            media: FontMediaDeclaration::Declared(match font.media_type {
                WireFontMediaType::SfntTrueTypeGlyf => FontMediaType::SfntTrueTypeGlyf,
                WireFontMediaType::TtcTrueTypeGlyf => FontMediaType::TtcTrueTypeGlyf,
            }),
        });
    }
    let mut images = Vec::new();
    images
        .try_reserve_exact(wire.images.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for (index, image) in wire.images.iter().enumerate() {
        if usize::try_from(image.image_id) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        images.push(StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(image.image_id),
            uri: PortablePath::new(image.uri.clone())
                .map_err(|_| StagingSemanticSyntaxError::InvalidResource)?,
            expected_sha256: parse_optional_hash(image.expected_sha256.as_deref())?,
            media: ImageMediaDeclaration::Declared(match image.media_type {
                WireImageMediaType::Png => ImageMediaType::Png,
            }),
        });
    }
    Ok(StagingM4ResourceCatalog { font_faces, images })
}

fn parse_optional_hash(
    value: Option<&str>,
) -> Result<Option<[u8; 32]>, StagingSemanticSyntaxError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.len() != 64 {
        return Err(StagingSemanticSyntaxError::InvalidResource);
    }
    let mut result = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        };
        result[index] = digit(pair[0])
            .and_then(|high| digit(pair[1]).map(|low| high * 16 + low))
            .ok_or(StagingSemanticSyntaxError::InvalidResource)?;
    }
    Ok(Some(result))
}

struct StagingSemanticStyleSheets {
    semantic: StyleSheet,
    ordinary: StyleSheet,
}

fn lower_semantic_style_rules(
    sheet: &WireStagingStyleSheet,
    limits: &ValidatedResourceLimits,
) -> Result<StagingSemanticStyleSheets, StagingSemanticSyntaxError> {
    let rules = &sheet.rules;
    if u64::try_from(rules.len()).map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?
        > limits.get().max_style_rules
    {
        return Err(StagingSemanticSyntaxError::InvalidStyle);
    }
    let mut parsed = Vec::new();
    let mut semantic_rules = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        let source_order = rule.source_order;
        if usize::try_from(source_order) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidStyle);
        }
        let selector = rule.selector.as_str();
        let mut parts = selector.split('.');
        let block_type = parts
            .next()
            .ok_or(StagingSemanticSyntaxError::InvalidStyle)?;
        if !matches!(
            block_type,
            "paragraph"
                | "heading"
                | "list"
                | "table"
                | "figure"
                | "page_break"
                | "semantic_container"
        ) {
            return Err(StagingSemanticSyntaxError::InvalidStyle);
        }
        let required_classes: Vec<String> = parts.map(str::to_owned).collect();
        validate_classes(&required_classes)?;
        let style_id = StyleId::new(rule.style_id.clone())
            .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        let extends = rule
            .extends
            .as_ref()
            .map(|value| {
                StyleId::new(value.clone()).map_err(|_| StagingSemanticSyntaxError::InvalidStyle)
            })
            .transpose()?;
        let mut typed_declarations = Vec::new();
        for declaration in &rule.declarations {
            let name = declaration.name.as_str();
            if BasicStyleProperty::from_str(name).is_none() {
                return Err(StagingSemanticSyntaxError::InvalidStyle);
            }
            typed_declarations.push(Declaration {
                name: name.to_owned(),
                value: lower_semantic_style_value(&declaration.value)?,
                important: declaration.important,
            });
        }
        let mapped_selector = if block_type == "semantic_container" {
            format!("paragraph{}", &selector["semantic_container".len()..])
        } else {
            selector.to_owned()
        };
        parsed.push(StyleRule {
            style_id,
            extends,
            selector: mapped_selector,
            source_order,
            declarations: typed_declarations,
        });
        semantic_rules.push(block_type == "semantic_container");
    }
    let validation_sheet = StyleSheet {
        rules: parsed.clone(),
    };
    validation_sheet
        .validate_table_document_styles()
        .map_err(map_semantic_style_error)?;

    let mut ordinary_rules = parsed.clone();
    for (index, rule) in ordinary_rules.iter_mut().enumerate() {
        if semantic_rules[index] {
            // This sheet is queried only for list/table/figure ancestors.
            // Keeping semantic rules on paragraph preserves `extends` edges
            // without inventing a sentinel class an authored block could use.
            rule.selector = "paragraph".to_owned();
        }
    }
    let ordinary = StyleSheet {
        rules: ordinary_rules,
    };
    ordinary
        .validate_table_document_styles()
        .map_err(map_semantic_style_error)?;

    let by_id: BTreeMap<&StyleId, usize> = parsed
        .iter()
        .enumerate()
        .map(|(index, rule)| (&rule.style_id, index))
        .collect();
    let mut included = BTreeSet::new();
    for (index, is_semantic) in semantic_rules.iter().copied().enumerate() {
        if !is_semantic {
            continue;
        }
        let mut current = Some(index);
        while let Some(rule_index) = current {
            if !included.insert(rule_index) {
                break;
            }
            current = parsed[rule_index]
                .extends
                .as_ref()
                .and_then(|parent| by_id.get(parent).copied());
        }
    }
    let mut cascade_rules = Vec::new();
    for (original_index, mut rule) in parsed.into_iter().enumerate() {
        if !included.contains(&original_index) {
            continue;
        }
        if rule.declarations.iter().any(|declaration| {
            matches!(
                BasicStyleProperty::from_str(&declaration.name),
                Some(BasicStyleProperty::Width | BasicStyleProperty::KeepCaption)
            )
        }) {
            return Err(StagingSemanticSyntaxError::InapplicableStyle);
        }
        if !semantic_rules[original_index] {
            // Semantic cascade is queried only as paragraph. A heading
            // selector keeps an ordinary ancestor available to `extends`
            // while making direct selector matching impossible.
            rule.selector = "heading".to_owned();
        }
        rule.source_order = u32::try_from(cascade_rules.len())
            .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        cascade_rules.push(rule);
    }
    let cascade_sheet = StyleSheet {
        rules: cascade_rules,
    };
    cascade_sheet
        .validate_basic_document_styles()
        .map_err(map_semantic_style_error)?;
    Ok(StagingSemanticStyleSheets {
        semantic: cascade_sheet,
        ordinary,
    })
}

fn lower_semantic_style_value(
    value: &WireStagingStyleValue,
) -> Result<StyleValue, StagingSemanticSyntaxError> {
    match value {
        WireStagingStyleValue::Keyword { value } => Ok(StyleValue::Keyword(value.clone())),
        WireStagingStyleValue::String { value } => Ok(StyleValue::Text(value.clone())),
        WireStagingStyleValue::Integer { value } => Ok(StyleValue::Integer(*value)),
        WireStagingStyleValue::Length { value } => Length::from_raw(*value)
            .map(StyleValue::Length)
            .ok_or(StagingSemanticSyntaxError::InvalidStyle),
        WireStagingStyleValue::Boolean { value } => Ok(StyleValue::Boolean(*value)),
        WireStagingStyleValue::FontFamilyList { families } => {
            Ok(StyleValue::FontFamilyList(families.clone()))
        }
        WireStagingStyleValue::Ratio {
            numerator,
            denominator,
        } => NonZeroU64::new(*denominator)
            .map(|denominator| StyleValue::Ratio {
                numerator: *numerator,
                denominator,
            })
            .ok_or(StagingSemanticSyntaxError::InvalidStyle),
    }
}

fn map_semantic_style_error(error: StyleValidationError) -> StagingSemanticSyntaxError {
    match error {
        StyleValidationError::InapplicableProperty => StagingSemanticSyntaxError::InapplicableStyle,
        _ => StagingSemanticSyntaxError::InvalidStyle,
    }
}

fn collect_computed_styles(
    blocks: &[StagingM4Block],
    rules: &StagingSemanticStyleSheets,
    parent: Option<&SemanticContainerInheritanceStyle>,
    output: &mut BTreeMap<NodeId, SemanticContainerComputedStyle>,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => {
                let kind = match semantic_kind {
                    SemanticContainerKind::Result => SemanticContainerStyleKind::Result,
                    SemanticContainerKind::Proof => SemanticContainerStyleKind::Proof,
                    SemanticContainerKind::Exercise => SemanticContainerStyleKind::Exercise,
                };
                let style = cascade_staging_semantic_container_style(
                    kind,
                    &common.classes,
                    &rules.semantic,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                let inheritance = style.inheritance_style().clone();
                if output.insert(common.node_id, style).is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
                }
                collect_computed_styles(blocks, rules, Some(&inheritance), output)?;
            }
            StagingM4Block::List { common, items } => {
                let inheritance = cascade_staging_semantic_descendant_style(
                    "list",
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                for item in items {
                    collect_computed_styles(&item.blocks, rules, Some(&inheritance), output)?;
                }
            }
            StagingM4Block::Table { common, head, body } => {
                let inheritance = cascade_staging_semantic_descendant_style(
                    "table",
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_computed_styles(&cell.blocks, rules, Some(&inheritance), output)?;
                }
            }
            StagingM4Block::Figure {
                common, caption, ..
            } => {
                let inheritance = cascade_staging_semantic_descendant_style(
                    "figure",
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                collect_computed_styles(caption, rules, Some(&inheritance), output)?
            }
            _ => {}
        }
    }
    Ok(())
}

fn encode_semantic_receipt(
    document: &StagingM4Document,
    resources: &StagingM4ResourceCatalog,
    styles: &BTreeMap<NodeId, SemanticContainerComputedStyle>,
    canonical_package: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, SEMANTIC_SYNTAX_FINGERPRINT_ALGORITHM);
    output.push_str(",\"canonical_package_sha256\":");
    push_hash(&mut output, canonical_package);
    output.push_str(",\"containers\":[");
    let mut first = true;
    encode_container_records(&document.blocks, styles, &mut first, &mut output);
    for footnote in &document.footnotes {
        encode_container_records(&footnote.blocks, styles, &mut first, &mut output);
    }
    output.push_str("],\"resources\":{");
    output.push_str("\"fonts\":[");
    for (index, font) in resources.font_faces.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"font_face_id\":");
        output.push_str(&font.font_face_id.get().to_string());
        output.push_str(",\"media_type\":");
        push_jcs_string(
            &mut output,
            match font.media {
                FontMediaDeclaration::Declared(value) => value.as_str(),
                FontMediaDeclaration::LegacyUnspecified => "legacy_unspecified",
            },
        );
        output.push('}');
    }
    output.push_str("],\"images\":[");
    for (index, image) in resources.images.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&image.image_id.get().to_string());
        output.push_str(",\"media_type\":");
        push_jcs_string(
            &mut output,
            match image.media {
                ImageMediaDeclaration::Declared(value) => value.as_str(),
                ImageMediaDeclaration::LegacyUnspecified => "legacy_unspecified",
            },
        );
        output.push('}');
    }
    output.push_str("]}}");
    output
}

fn encode_container_records(
    blocks: &[StagingM4Block],
    styles: &BTreeMap<NodeId, SemanticContainerComputedStyle>,
    first: &mut bool,
    output: &mut String,
) {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => {
                if !*first {
                    output.push(',');
                }
                *first = false;
                let style = &styles[&common.node_id];
                output.push_str("{\"child_node_ids\":[");
                for (index, child) in blocks.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    output.push_str(&child.node_id().get().to_string());
                }
                output.push_str("],\"classes\":[");
                for (index, class) in common.classes.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    push_jcs_string(output, class);
                }
                output.push_str("],\"kind\":");
                push_jcs_string(output, semantic_kind.as_str());
                output.push_str(",\"node_id\":");
                output.push_str(&common.node_id.get().to_string());
                output.push_str(",\"source_span\":{");
                output.push_str("\"end_byte\":");
                output.push_str(&common.span.end_byte().get().to_string());
                output.push_str(",\"source_id\":");
                output.push_str(&common.span.source_id().get().to_string());
                output.push_str(",\"start_byte\":");
                output.push_str(&common.span.start_byte().get().to_string());
                output.push_str("},\"style\":{");
                let block_style = style.block_style();
                output.push_str("\"end_indent\":");
                output.push_str(&block_style.end_indent().get().raw().to_string());
                output.push_str(",\"font_families\":");
                match style.inheritance_style().font_families() {
                    Some(families) => {
                        output.push('[');
                        for (index, family) in families.iter().enumerate() {
                            if index > 0 {
                                output.push(',');
                            }
                            push_jcs_string(output, family);
                        }
                        output.push(']');
                    }
                    None => output.push_str("null"),
                }
                output.push_str(",\"font_size\":");
                match style.inheritance_style().font_size() {
                    Some(value) => output.push_str(&value.get().raw().to_string()),
                    None => output.push_str("null"),
                }
                output.push_str(",\"keep_with_next\":");
                output.push_str(if block_style.keep_with_next() {
                    "true"
                } else {
                    "false"
                });
                output.push_str(",\"line_height\":");
                match style.inheritance_style().line_height() {
                    Some(value) => output.push_str(&value.get().raw().to_string()),
                    None => output.push_str("null"),
                }
                output.push_str(",\"page\":");
                match style.page_name() {
                    Some(value) => push_jcs_string(output, value.as_str()),
                    None => output.push_str("null"),
                }
                output.push_str(",\"space_after\":");
                output.push_str(&block_style.space_after().get().raw().to_string());
                output.push_str(",\"space_before\":");
                output.push_str(&block_style.space_before().get().raw().to_string());
                output.push_str(",\"start_indent\":");
                output.push_str(&block_style.start_indent().get().raw().to_string());
                output.push_str(",\"text_align\":");
                push_jcs_string(output, block_style.text_align().as_str());
                output.push_str("}}");
                encode_container_records(blocks, styles, first, output);
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    encode_container_records(&item.blocks, styles, first, output);
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    encode_container_records(&cell.blocks, styles, first, output);
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                encode_container_records(caption, styles, first, output)
            }
            _ => {}
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_document_package::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireStagingM4Inline, WireStagingStyleDeclaration,
        WireStagingStyleValue,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));

    fn parse(bytes: &[u8]) -> Result<ValidatedStagingSemanticPackage, Box<dyn std::error::Error>> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes, &DocumentPackageDecodePolicy::new(&limits))?;
        Ok(StagingSemanticPackageParser::new().parse(decoded, &limits)?)
    }

    fn mutate_and_encode(update: impl FnOnce(&mut WireStagingM4DocumentPackage)) -> Vec<u8> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        update(&mut wire);
        StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap()
            .into_bytes()
    }

    #[test]
    fn semantic_container_validates_ownership_style_and_typed_round_trip() {
        let package = parse(FIXTURE).unwrap();
        assert_eq!(package.semantic_container_count(), 3);
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .semantic_kind(),
            SemanticContainerStyleKind::Result
        );
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .block_style()
                .space_before()
                .get()
                .raw(),
            7
        );
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(package.checked_wire().unwrap())
            .unwrap();
        let reparsed = parse(encoded.as_bytes()).unwrap();
        assert_eq!(
            package.semantic_fingerprint(),
            reparsed.semantic_fingerprint()
        );
    }

    #[test]
    fn semantic_container_decode_and_syntax_limits_are_one_receipted_input() {
        let decode_limits =
            ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&decode_limits))
            .unwrap();
        let mut different = typaxis_core::ResourceLimits::default();
        different.max_pages -= 1;
        let different = ValidatedResourceLimits::new(different).unwrap();
        assert!(matches!(
            StagingSemanticPackageParser::new().parse(decoded, &different),
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        ));
    }

    #[test]
    fn semantic_container_recursive_empty_reaches_profile_boundary_but_bad_owner_and_style_do_not()
    {
        fn remove_inline_content(inline: &mut WireStagingM4Inline) {
            match inline {
                WireStagingM4Inline::Text { node_id, span, .. }
                | WireStagingM4Inline::Reference { node_id, span, .. }
                | WireStagingM4Inline::FootnoteReference { node_id, span, .. } => {
                    *inline = WireStagingM4Inline::HardBreak {
                        node_id: *node_id,
                        span: *span,
                    };
                }
                WireStagingM4Inline::Emphasis { children, .. }
                | WireStagingM4Inline::Strong { children, .. }
                | WireStagingM4Inline::Link { children, .. } => {
                    children.iter_mut().for_each(remove_inline_content);
                }
                WireStagingM4Inline::Anchor { .. }
                | WireStagingM4Inline::SoftBreak { .. }
                | WireStagingM4Inline::HardBreak { .. } => {}
            }
        }

        fn remove_block_content(blocks: &mut [WireStagingM4Block]) {
            for block in blocks {
                match block {
                    WireStagingM4Block::Paragraph { children, .. }
                    | WireStagingM4Block::Heading { children, .. } => {
                        children.iter_mut().for_each(remove_inline_content);
                    }
                    WireStagingM4Block::List { items, .. } => {
                        for item in items {
                            remove_block_content(&mut item.blocks);
                        }
                    }
                    WireStagingM4Block::Table { head, body, .. } => {
                        for cell in head.iter_mut().chain(body).flat_map(|row| &mut row.cells) {
                            remove_block_content(&mut cell.blocks);
                        }
                    }
                    WireStagingM4Block::Figure { caption, .. }
                    | WireStagingM4Block::SemanticContainer {
                        blocks: caption, ..
                    } => remove_block_content(caption),
                    WireStagingM4Block::PageBreak { .. } => {}
                }
            }
        }

        let empty = mutate_and_encode(|wire| {
            let mut document = wire.document().clone();
            remove_block_content(&mut document.blocks);
            wire.replace_typed_regions(document, wire.resources().clone());
        });
        assert!(parse(&empty).is_ok());

        let foreign = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"end_byte\":6,\"source_id\":0,\"start_byte\":0},\"text_span\"",
            "\"end_byte\":20,\"source_id\":0,\"start_byte\":0},\"text_span\"",
            1,
        );
        assert!(parse(foreign.as_bytes()).is_err());

        let width = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"name\":\"space_before\"",
            "\"name\":\"width\"",
            1,
        );
        assert!(parse(width.as_bytes()).is_err());
    }

    #[test]
    fn semantic_container_style_honors_important_extends_and_rejects_prefix_aliases() {
        let inherited = mutate_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[0].declarations[0].important = true;
            sheet.rules[1].extends = Some("semantic-base".to_owned());
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&inherited).unwrap();
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .block_style()
                .space_before()
                .get()
                .raw(),
            2
        );

        let malformed = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"selector\":\"semantic_container\"",
            "\"selector\":\"semantic_container_alias\"",
            1,
        );
        assert!(parse(malformed.as_bytes()).is_err());

        let unknown_parent = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"extends\":null,\"selector\":\"semantic_container.feature\"",
            "\"extends\":\"missing\",\"selector\":\"semantic_container.feature\"",
            1,
        );
        assert!(parse(unknown_parent.as_bytes()).is_err());

        let inherited_align = mutate_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[1]
                .declarations
                .push(WireStagingStyleDeclaration {
                    important: false,
                    name: "text_align".to_owned(),
                    value: WireStagingStyleValue::Keyword {
                        value: "end".to_owned(),
                    },
                });
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&inherited_align).unwrap();
        assert_eq!(
            package
                .computed_style(NodeId::new(4))
                .unwrap()
                .block_style()
                .text_align()
                .as_str(),
            "end"
        );

        let inherited_text = mutate_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[1].declarations.extend([
                WireStagingStyleDeclaration {
                    important: false,
                    name: "font_family".to_owned(),
                    value: WireStagingStyleValue::FontFamilyList {
                        families: vec!["Body".to_owned()],
                    },
                },
                WireStagingStyleDeclaration {
                    important: false,
                    name: "font_size".to_owned(),
                    value: WireStagingStyleValue::Length { value: 10 },
                },
                WireStagingStyleDeclaration {
                    important: false,
                    name: "line_height".to_owned(),
                    value: WireStagingStyleValue::Length { value: 12 },
                },
                WireStagingStyleDeclaration {
                    important: false,
                    name: "page".to_owned(),
                    value: WireStagingStyleValue::String {
                        value: "chapter".to_owned(),
                    },
                },
            ]);
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&inherited_text).unwrap();
        let result = package.computed_style(NodeId::new(1)).unwrap();
        let proof = package.computed_style(NodeId::new(4)).unwrap();
        assert_eq!(
            result.inheritance_style().font_families().unwrap(),
            ["Body"]
        );
        assert_eq!(proof.inheritance_style().font_families().unwrap(), ["Body"]);
        assert_eq!(
            proof.inheritance_style().font_size().unwrap().get().raw(),
            10
        );
        assert_eq!(
            proof.inheritance_style().line_height().unwrap().get().raw(),
            12
        );
        assert_eq!(result.page_name().unwrap().as_str(), "chapter");
        assert!(proof.page_name().is_none());

        let isolated_extends = mutate_and_encode(|wire| {
            let mut document = wire.document().clone();
            let WireStagingM4Block::SemanticContainer { classes, .. } = &mut document.blocks[0]
            else {
                panic!("fixture root must remain semantic")
            };
            classes.insert(0, "__m4_inheritance_only".to_owned());
            let resources = wire.resources().clone();
            wire.replace_typed_regions(document, resources);

            let mut sheet = wire.style_sheet().clone();
            let mut ordinary_parent = sheet.rules[0].clone();
            ordinary_parent.style_id = "ordinary-parent".to_owned();
            ordinary_parent.extends = None;
            ordinary_parent.selector = "paragraph".to_owned();
            ordinary_parent.source_order = 2;
            ordinary_parent.declarations = vec![WireStagingStyleDeclaration {
                important: false,
                name: "space_after".to_owned(),
                value: WireStagingStyleValue::Length { value: 99 },
            }];
            let mut nested = sheet.rules[0].clone();
            nested.style_id = "semantic-nested".to_owned();
            nested.extends = Some("ordinary-parent".to_owned());
            nested.selector = "semantic_container.nested".to_owned();
            nested.source_order = 3;
            nested.declarations = vec![WireStagingStyleDeclaration {
                important: false,
                name: "space_before".to_owned(),
                value: WireStagingStyleValue::Length { value: 2 },
            }];
            sheet.rules.extend([ordinary_parent, nested]);
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&isolated_extends).unwrap();
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .block_style()
                .space_after()
                .get()
                .raw(),
            3
        );
        assert_eq!(
            package
                .computed_style(NodeId::new(4))
                .unwrap()
                .block_style()
                .space_after()
                .get()
                .raw(),
            99
        );

        for (name, value) in [
            (
                "width",
                WireStagingStyleValue::Keyword {
                    value: "auto".to_owned(),
                },
            ),
            (
                "keep_caption",
                WireStagingStyleValue::Boolean { value: true },
            ),
        ] {
            let inapplicable = mutate_and_encode(|wire| {
                let mut sheet = wire.style_sheet().clone();
                sheet.rules[0].declarations[0].name = name.to_owned();
                sheet.rules[0].declarations[0].value = value;
                wire.replace_style_sheet(sheet);
            });
            assert!(matches!(
                parse(&inapplicable),
                Err(error) if error.to_string().contains("inapplicable")
            ));
        }
    }
}
