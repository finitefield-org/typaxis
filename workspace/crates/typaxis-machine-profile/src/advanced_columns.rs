use std::sync::Arc;
use typaxis_core::{
    push_jcs_string, sha256, DocumentFingerprint, MasterId, NodeId, Rect, StyleFingerprint,
    ValidatedResourceLimits,
};
use typaxis_syntax::machine_profile_boundary::{
    Block, ColumnBalance, ColumnFill, ColumnLayout, FigurePlacement, Inline, StyleValue,
};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub const STAGING_COLUMNS_PROFILE_ID: &str = "typaxis.machine-pdf/columns-1";
pub const COLUMNS_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.columns-profile-receipt/1";

#[derive(Clone)]
pub struct StagingColumnsSessionIdentity(Arc<()>);

impl StagingColumnsSessionIdentity {
    /// Allocate an opaque identity for one invocation of the crate-private
    /// advanced-pagination runner.
    pub fn fresh() -> Self {
        Self(Arc::new(()))
    }
}

impl PartialEq for StagingColumnsSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StagingColumnsSessionIdentity {}

impl std::fmt::Debug for StagingColumnsSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingColumnsSessionIdentity(..)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingColumnsProfileDescriptor;

impl StagingColumnsProfileDescriptor {
    pub const PROFILE_ID: &'static str = STAGING_COLUMNS_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.3";

    pub const fn supports_sequential_columns(self) -> bool {
        true
    }

    pub const fn supports_final_page_balance(self) -> bool {
        true
    }

    pub const fn writing_mode(self) -> &'static str {
        "horizontal-tb"
    }

    pub const fn page_progression(self) -> &'static str {
        "ltr"
    }

    pub const fn requires_full_media_trim(self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingColumnsPreflightError {
    UnsupportedContent(NodeId),
    UnsupportedStyle,
    UnsupportedMaster(Option<MasterId>),
    InvalidGeometry(MasterId),
    MissingTextFont,
    ReceiptMismatch,
    ArithmeticOverflow,
}

impl std::fmt::Display for StagingColumnsPreflightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContent(node) => {
                write!(
                    formatter,
                    "L5100: unsupported columns content at node {}",
                    node.get()
                )
            }
            Self::UnsupportedStyle => formatter.write_str("L5101: unsupported columns style"),
            Self::UnsupportedMaster(master) => match master {
                Some(master) => write!(formatter, "L5101: unsupported columns master {master}"),
                None => formatter.write_str("L5101: unsupported columns master-set form"),
            },
            Self::InvalidGeometry(master) => {
                write!(formatter, "L5101: invalid column geometry for {master}")
            }
            Self::MissingTextFont => formatter.write_str("L5101: text content requires a font"),
            Self::ReceiptMismatch => formatter.write_str("I9190: columns receipt mismatch"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: columns arithmetic overflow"),
        }
    }
}

impl std::error::Error for StagingColumnsPreflightError {}

/// Non-forgeable proof of the complete private columns profile gate.
#[derive(Debug)]
pub struct StagingColumnsPreflightReceipt {
    document: DocumentFingerprint,
    style: StyleFingerprint,
    raw_package_sha256: [u8; 32],
    canonical_package_sha256: [u8; 32],
    limits: ValidatedResourceLimits,
    session: StagingColumnsSessionIdentity,
    profile_receipt_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingColumnsPreflightReceipt {
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn raw_package_sha256(&self) -> [u8; 32] {
        self.raw_package_sha256
    }

    pub const fn canonical_package_sha256(&self) -> [u8; 32] {
        self.canonical_package_sha256
    }

    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }

    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingAdvancedPackage,
        limits: &ValidatedResourceLimits,
        session: &StagingColumnsSessionIdentity,
    ) -> Result<(), StagingColumnsPreflightError> {
        let epoch = package.package().epoch_identity();
        if self.document != epoch.document()
            || self.style != epoch.style()
            || self.raw_package_sha256 != package.raw_sha256()
            || self.canonical_package_sha256 != package.canonical_jcs_sha256()
            || self.limits != *limits
            || self.session != *session
            || self.canonical_jcs != encode_receipt(package, limits)
            || self.profile_receipt_sha256 != sha256(self.canonical_jcs.as_bytes())
        {
            return Err(StagingColumnsPreflightError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_columns_profile(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingColumnsSessionIdentity,
) -> Result<StagingColumnsPreflightReceipt, StagingColumnsPreflightError> {
    validate_document_domain(package)?;
    validate_style_domain(package)?;
    validate_master_form(package)?;

    if document_has_text(&package.package().package().document.blocks)
        && package.package().package().resources.font_faces.is_empty()
    {
        return Err(StagingColumnsPreflightError::MissingTextFont);
    }

    let canonical_jcs = encode_receipt(package, limits);
    Ok(StagingColumnsPreflightReceipt {
        document: package.package().epoch_identity().document(),
        style: package.package().epoch_identity().style(),
        raw_package_sha256: package.raw_sha256(),
        canonical_package_sha256: package.canonical_jcs_sha256(),
        limits: limits.clone(),
        session: session.clone(),
        profile_receipt_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn validate_document_domain(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingColumnsPreflightError> {
    let document = &package.package().package().document;
    if let Some(footnote) = document.footnotes.first() {
        return Err(StagingColumnsPreflightError::UnsupportedContent(
            footnote.node_id,
        ));
    }
    let mut stack: Vec<&Block> = document.blocks.iter().rev().collect();
    while let Some(block) = stack.pop() {
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                validate_body_inlines(children)?;
            }
            Block::List { items, .. } => {
                for nested in items.iter().rev().flat_map(|item| item.blocks.iter().rev()) {
                    stack.push(nested);
                }
            }
            Block::Figure {
                node_id, caption, ..
            } => {
                if package.figure_placement(*node_id) != Some(FigurePlacement::Block) {
                    return Err(StagingColumnsPreflightError::UnsupportedContent(*node_id));
                }
                stack.extend(caption.iter().rev());
            }
            Block::PageBreak { .. } => {}
            Block::Table { node_id, .. } => {
                return Err(StagingColumnsPreflightError::UnsupportedContent(*node_id));
            }
        }
    }
    Ok(())
}

fn validate_body_inlines(inlines: &[Inline]) -> Result<(), StagingColumnsPreflightError> {
    let mut stack: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = stack.pop() {
        match inline {
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
            Inline::Link { children, .. } => stack.extend(children.iter().rev()),
            Inline::Emphasis { node_id, .. }
            | Inline::Strong { node_id, .. }
            | Inline::FootnoteReference { node_id, .. } => {
                return Err(StagingColumnsPreflightError::UnsupportedContent(*node_id));
            }
        }
    }
    Ok(())
}

fn validate_style_domain(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingColumnsPreflightError> {
    package
        .package()
        .package()
        .style_sheet
        .validate_basic_document_styles()
        .map_err(|_| StagingColumnsPreflightError::UnsupportedStyle)?;
    for rule in &package.package().package().style_sheet.rules {
        for declaration in &rule.declarations {
            if declaration.name == "page"
                && !matches!(&declaration.value, StyleValue::Keyword(value) if value == "auto")
            {
                return Err(StagingColumnsPreflightError::UnsupportedStyle);
            }
        }
    }
    Ok(())
}

fn validate_master_form(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingColumnsPreflightError> {
    let base = &package.package().package().page_masters;
    let advanced = package.page_masters();
    if base.masters.len() != 1
        || !base.selection_rules.is_empty()
        || base.default_master_id != base.masters[0].master_id
        || advanced.masters.len() != 1
    {
        return Err(StagingColumnsPreflightError::UnsupportedMaster(None));
    }
    let master = &base.masters[0];
    let extension = &advanced.masters[0];
    if master.master_id != extension.master_id
        || master.header.is_some()
        || master.footer.is_some()
        || master.footnote.is_some()
        || extension.header_content.is_some()
        || extension.footer_content.is_some()
    {
        return Err(StagingColumnsPreflightError::UnsupportedMaster(Some(
            master.master_id.clone(),
        )));
    }
    validate_geometry(
        master.width.get().raw(),
        master.height.get().raw(),
        extension.trim,
    )
    .and_then(|()| validate_column_form(master.body, extension.column_layout.as_ref()))
    .map_err(|_| StagingColumnsPreflightError::InvalidGeometry(master.master_id.clone()))
}

fn validate_geometry(width: i64, height: i64, trim: Rect) -> Result<(), ()> {
    if trim.x().raw() != 0
        || trim.y().raw() != 0
        || trim.width().get().raw() != width
        || trim.height().get().raw() != height
    {
        return Err(());
    }
    Ok(())
}

fn validate_column_form(body: Rect, layout: Option<&ColumnLayout>) -> Result<(), ()> {
    let Some(layout) = layout else { return Ok(()) };
    if layout.fill != ColumnFill::Sequential || layout.balance != ColumnBalance::LastPage {
        return Err(());
    }
    let count = i64::from(layout.count.get());
    let total_gap = count
        .checked_sub(1)
        .and_then(|value| value.checked_mul(layout.gap.get().raw()))
        .ok_or(())?;
    let available = body.width().get().raw().checked_sub(total_gap).ok_or(())?;
    if available < count {
        return Err(());
    }
    Ok(())
}

fn document_has_text(blocks: &[Block]) -> bool {
    blocks.iter().any(|block| match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
            inlines_have_text(children)
        }
        Block::List { items, .. } => items.iter().any(|item| document_has_text(&item.blocks)),
        Block::Figure { caption, .. } => document_has_text(caption),
        Block::Table { head, body, .. } => head
            .iter()
            .chain(body)
            .flat_map(|row| &row.cells)
            .any(|cell| document_has_text(&cell.blocks)),
        Block::PageBreak { .. } => false,
    })
}

fn inlines_have_text(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text { .. } | Inline::Reference { .. } | Inline::FootnoteReference { .. } => true,
        Inline::Emphasis { children, .. }
        | Inline::Strong { children, .. }
        | Inline::Link { children, .. } => inlines_have_text(children),
        Inline::Anchor { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => false,
    })
}

fn encode_receipt(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, COLUMNS_PROFILE_RECEIPT_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.3\",\"document_sha256\":");
    push_hex(
        &mut output,
        package.package().epoch_identity().document().bytes(),
    );
    output.push_str(",\"effective_limits\":{");
    push_limits(&mut output, limits);
    output.push('}');
    output.push_str(",\"package_jcs_sha256\":");
    push_hex(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"package_sha256\":");
    push_hex(&mut output, package.raw_sha256());
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, STAGING_COLUMNS_PROFILE_ID);
    output.push_str(",\"style_sha256\":");
    push_hex(
        &mut output,
        package.package().epoch_identity().style().bytes(),
    );
    output.push('}');
    output
}

fn push_limits(output: &mut String, limits: &ValidatedResourceLimits) {
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

fn push_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}
