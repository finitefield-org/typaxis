use std::sync::Arc;
use typaxis_core::{
    push_jcs_string, sha256, DocumentFingerprint, MasterId, NodeId, Rect, StyleFingerprint,
    ValidatedResourceLimits,
};
use typaxis_syntax::machine_profile_boundary::{
    Block, FigurePlacement, Inline, PageMaster, PageParity, PageRegionBlock, PageRegionInline,
    StyleValue,
};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub const STAGING_HEADER_FOOTER_PROFILE_ID: &str = "typaxis.machine-pdf/header-footer-1";
pub const HEADER_FOOTER_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.header-footer-profile-receipt/1";

#[derive(Clone)]
pub struct StagingHeaderFooterSessionIdentity(Arc<()>);

impl StagingHeaderFooterSessionIdentity {
    /// Allocate an opaque identity for one invocation of the crate-private
    /// advanced-pagination runner.
    pub fn fresh() -> Self {
        Self(Arc::new(()))
    }
}

impl PartialEq for StagingHeaderFooterSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StagingHeaderFooterSessionIdentity {}

impl std::fmt::Debug for StagingHeaderFooterSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingHeaderFooterSessionIdentity(..)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMasterSelectionCapability {
    Single,
    FirstLeftRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingHeaderFooterProfileDescriptor;

impl StagingHeaderFooterProfileDescriptor {
    pub const PROFILE_ID: &'static str = STAGING_HEADER_FOOTER_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.3";

    pub const fn supports_custom_trim(self) -> bool {
        true
    }

    pub const fn supports_header_footer(self) -> bool {
        true
    }

    pub const fn writing_mode(self) -> &'static str {
        "horizontal-tb"
    }

    pub const fn page_progression(self) -> &'static str {
        "ltr"
    }

    pub const fn page_boxes(self) -> &'static [&'static str] {
        &["crop", "media", "trim"]
    }

    pub const fn master_selection(self) -> &'static [StagingMasterSelectionCapability] {
        &[
            StagingMasterSelectionCapability::Single,
            StagingMasterSelectionCapability::FirstLeftRight,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingHeaderFooterPreflightError {
    UnsupportedContent(NodeId),
    UnsupportedStyle,
    UnsupportedMaster(Option<MasterId>),
    InvalidGeometry(MasterId),
    MissingTextFont,
    ReceiptMismatch,
    ArithmeticOverflow,
}

impl std::fmt::Display for StagingHeaderFooterPreflightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContent(node) => {
                write!(
                    formatter,
                    "L5100: unsupported header-footer content at node {}",
                    node.get()
                )
            }
            Self::UnsupportedStyle => formatter.write_str("L5101: unsupported header-footer style"),
            Self::UnsupportedMaster(master) => match master {
                Some(master) => write!(formatter, "L5101: unsupported page master {master}"),
                None => formatter.write_str("L5101: unsupported page-master selection form"),
            },
            Self::InvalidGeometry(master) => {
                write!(
                    formatter,
                    "L5101: invalid page-master geometry for {master}"
                )
            }
            Self::MissingTextFont => formatter.write_str("L5101: text content requires a font"),
            Self::ReceiptMismatch => formatter.write_str("I9190: header-footer receipt mismatch"),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5101: page-master arithmetic overflow")
            }
        }
    }
}

impl std::error::Error for StagingHeaderFooterPreflightError {}

/// Non-forgeable proof of the complete private header/footer profile gate.
#[derive(Debug)]
pub struct StagingHeaderFooterPreflightReceipt {
    document: DocumentFingerprint,
    style: StyleFingerprint,
    raw_package_sha256: [u8; 32],
    canonical_package_sha256: [u8; 32],
    limits: ValidatedResourceLimits,
    session: StagingHeaderFooterSessionIdentity,
    profile_receipt_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingHeaderFooterPreflightReceipt {
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

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingAdvancedPackage,
        limits: &ValidatedResourceLimits,
        session: &StagingHeaderFooterSessionIdentity,
    ) -> Result<(), StagingHeaderFooterPreflightError> {
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
            return Err(StagingHeaderFooterPreflightError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_header_footer_profile(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingHeaderFooterSessionIdentity,
) -> Result<StagingHeaderFooterPreflightReceipt, StagingHeaderFooterPreflightError> {
    validate_document_domain(package)?;
    validate_style_domain(package)?;
    validate_master_form(package)?;

    let text_present = document_has_text(&package.package().package().document.blocks)
        || package.page_masters().masters.iter().any(|master| {
            master
                .header_content
                .as_ref()
                .is_some_and(|region| region_has_text(&region.blocks))
                || master
                    .footer_content
                    .as_ref()
                    .is_some_and(|region| region_has_text(&region.blocks))
        });
    if text_present && package.package().package().resources.font_faces.is_empty() {
        return Err(StagingHeaderFooterPreflightError::MissingTextFont);
    }

    let canonical_jcs = encode_receipt(package, limits);
    Ok(StagingHeaderFooterPreflightReceipt {
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
) -> Result<(), StagingHeaderFooterPreflightError> {
    let document = &package.package().package().document;
    if let Some(footnote) = document.footnotes.first() {
        return Err(StagingHeaderFooterPreflightError::UnsupportedContent(
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
                    return Err(StagingHeaderFooterPreflightError::UnsupportedContent(
                        *node_id,
                    ));
                }
                stack.extend(caption.iter().rev());
            }
            Block::PageBreak { .. } => {}
            Block::Table { node_id, .. } => {
                return Err(StagingHeaderFooterPreflightError::UnsupportedContent(
                    *node_id,
                ));
            }
        }
    }
    Ok(())
}

fn validate_body_inlines(inlines: &[Inline]) -> Result<(), StagingHeaderFooterPreflightError> {
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
                return Err(StagingHeaderFooterPreflightError::UnsupportedContent(
                    *node_id,
                ));
            }
        }
    }
    Ok(())
}

fn validate_style_domain(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingHeaderFooterPreflightError> {
    package
        .package()
        .package()
        .style_sheet
        .validate_basic_document_styles()
        .map_err(|_| StagingHeaderFooterPreflightError::UnsupportedStyle)?;
    for rule in &package.package().package().style_sheet.rules {
        for declaration in &rule.declarations {
            if declaration.name == "page"
                && !matches!(&declaration.value, StyleValue::Keyword(value) if value == "auto")
            {
                return Err(StagingHeaderFooterPreflightError::UnsupportedStyle);
            }
        }
    }
    Ok(())
}

fn validate_master_form(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingHeaderFooterPreflightError> {
    let base = &package.package().package().page_masters;
    let advanced = package.page_masters();
    if advanced.masters.len() != base.masters.len() {
        return Err(StagingHeaderFooterPreflightError::UnsupportedMaster(None));
    }
    match base.masters.len() {
        1 if base.selection_rules.is_empty()
            && base.default_master_id == base.masters[0].master_id => {}
        3 if base.selection_rules.len() == 2 => {
            let first = &base.selection_rules[0];
            let left = &base.selection_rules[1];
            if first.source_order != 0
                || first.first != Some(true)
                || first.parity != PageParity::Any
                || first.named_page.is_some()
                || left.source_order != 1
                || left.first.is_some()
                || left.parity != PageParity::Even
                || left.named_page.is_some()
                || first.master_id == left.master_id
                || first.master_id == base.default_master_id
                || left.master_id == base.default_master_id
            {
                return Err(StagingHeaderFooterPreflightError::UnsupportedMaster(None));
            }
        }
        _ => return Err(StagingHeaderFooterPreflightError::UnsupportedMaster(None)),
    }

    for (master, extension) in base.masters.iter().zip(&advanced.masters) {
        if master.master_id != extension.master_id
            || master.footnote.is_some()
            || extension.column_layout.is_some()
            || master.header.is_some() != extension.header_content.is_some()
            || master.footer.is_some() != extension.footer_content.is_some()
        {
            return Err(StagingHeaderFooterPreflightError::UnsupportedMaster(Some(
                master.master_id.clone(),
            )));
        }
        validate_geometry(master, extension.trim).map_err(|_| {
            StagingHeaderFooterPreflightError::InvalidGeometry(master.master_id.clone())
        })?;
    }
    Ok(())
}

fn validate_geometry(master: &PageMaster, trim: Rect) -> Result<(), ()> {
    let media = (0, 0, master.width.get().raw(), master.height.get().raw());
    let trim_edges = rect_edges(trim).ok_or(())?;
    if trim_edges.0 < media.0
        || trim_edges.1 < media.1
        || trim_edges.2 > media.2
        || trim_edges.3 > media.3
    {
        return Err(());
    }
    let body = rect_edges(master.body).ok_or(())?;
    if body.0 < trim_edges.0
        || body.1 < trim_edges.1
        || body.2 > trim_edges.2
        || body.3 > trim_edges.3
    {
        return Err(());
    }
    if let Some(header) = master.header {
        let header = rect_edges(header).ok_or(())?;
        if header.0 != body.0 || header.2 != body.2 || header.1 < trim_edges.1 || header.3 > body.1
        {
            return Err(());
        }
    }
    if let Some(footer) = master.footer {
        let footer = rect_edges(footer).ok_or(())?;
        if footer.0 != body.0 || footer.2 != body.2 || footer.1 < body.3 || footer.3 > trim_edges.3
        {
            return Err(());
        }
    }
    let height = master.height.get().raw();
    height.checked_sub(trim_edges.3).ok_or(())?;
    height.checked_sub(trim_edges.1).ok_or(())?;
    Ok(())
}

fn rect_edges(rect: Rect) -> Option<(i64, i64, i64, i64)> {
    Some((
        rect.x().raw(),
        rect.y().raw(),
        rect.x().raw().checked_add(rect.width().get().raw())?,
        rect.y().raw().checked_add(rect.height().get().raw())?,
    ))
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

fn region_has_text(blocks: &[PageRegionBlock]) -> bool {
    blocks.iter().any(|block| {
        block
            .children()
            .iter()
            .any(|inline| matches!(inline, PageRegionInline::Text { .. }))
    })
}

fn encode_receipt(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, HEADER_FOOTER_PROFILE_RECEIPT_ALGORITHM);
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
    push_jcs_string(&mut output, STAGING_HEADER_FOOTER_PROFILE_ID);
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
