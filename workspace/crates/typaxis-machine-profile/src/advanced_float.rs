use std::sync::Arc;
use typaxis_core::{
    push_jcs_string, sha256, DocumentFingerprint, MasterId, NodeId, Rect, StyleFingerprint,
    ValidatedResourceLimits,
};
use typaxis_syntax::machine_profile_boundary::{
    BasicStyleBlockKind, Block, ColumnBalance, ColumnFill, ColumnLayout, FigurePlacement, Inline,
    MachineFigureWidth, ReferenceFormat, StyleValue,
};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub type StagingFloatPlacementClass = typaxis_syntax::machine_profile_boundary::FloatPlacementClass;

pub const STAGING_FLOAT_PROFILE_ID: &str = "typaxis.machine-pdf/float-1";
pub const FLOAT_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.float-profile-receipt/1";

#[derive(Clone)]
pub struct StagingFloatSessionIdentity(Arc<()>);

impl StagingFloatSessionIdentity {
    /// Allocate an opaque identity for one private advanced-pagination run.
    pub fn fresh() -> Self {
        Self(Arc::new(()))
    }
}

impl PartialEq for StagingFloatSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StagingFloatSessionIdentity {}

impl std::fmt::Debug for StagingFloatSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingFloatSessionIdentity(..)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingFloatProfileDescriptor;

impl StagingFloatProfileDescriptor {
    pub const PROFILE_ID: &'static str = STAGING_FLOAT_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.3";

    pub const fn supports_sequential_columns(self) -> bool {
        true
    }

    pub const fn supports_column_balance(self) -> bool {
        false
    }

    pub const fn supports_text_wrap(self) -> bool {
        false
    }

    pub const fn placement_classes(self) -> &'static [StagingFloatPlacementClass; 4] {
        &StagingFloatPlacementClass::ORDERED
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
pub enum StagingFloatPreflightError {
    UnsupportedContent(NodeId),
    NestedFloat(NodeId),
    UnsupportedStyle,
    UnsupportedMaster(Option<MasterId>),
    InvalidGeometry(MasterId),
    MissingTextFont,
    ReceiptMismatch,
    ArithmeticOverflow,
}

impl std::fmt::Display for StagingFloatPreflightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContent(node) => {
                write!(
                    formatter,
                    "L5100: unsupported float content at node {}",
                    node.get()
                )
            }
            Self::NestedFloat(node) => write!(
                formatter,
                "L5100: floating Figure at node {} is not a direct body child",
                node.get()
            ),
            Self::UnsupportedStyle => formatter.write_str("L5101: unsupported float style"),
            Self::UnsupportedMaster(master) => match master {
                Some(master) => write!(formatter, "L5101: unsupported float master {master}"),
                None => formatter.write_str("L5101: unsupported float master-set form"),
            },
            Self::InvalidGeometry(master) => {
                write!(formatter, "L5101: invalid float geometry for {master}")
            }
            Self::MissingTextFont => formatter.write_str("L5101: text content requires a font"),
            Self::ReceiptMismatch => formatter.write_str("I9190: float profile receipt mismatch"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: float arithmetic overflow"),
        }
    }
}

impl std::error::Error for StagingFloatPreflightError {}

/// Non-forgeable proof of the complete private `float-1` capability gate.
#[derive(Debug)]
pub struct StagingFloatPreflightReceipt {
    document: DocumentFingerprint,
    style: StyleFingerprint,
    raw_package_sha256: [u8; 32],
    canonical_package_sha256: [u8; 32],
    limits: ValidatedResourceLimits,
    session: StagingFloatSessionIdentity,
    profile_receipt_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingFloatPreflightReceipt {
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
        session: &StagingFloatSessionIdentity,
    ) -> Result<(), StagingFloatPreflightError> {
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
            return Err(StagingFloatPreflightError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_float_profile(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingFloatSessionIdentity,
) -> Result<StagingFloatPreflightReceipt, StagingFloatPreflightError> {
    validate_document_domain(package)?;
    validate_style_domain(package)?;
    validate_master_form(package)?;
    if document_has_text(&package.package().package().document.blocks)
        && package.package().package().resources.font_faces.is_empty()
    {
        return Err(StagingFloatPreflightError::MissingTextFont);
    }
    let canonical_jcs = encode_receipt(package, limits);
    Ok(StagingFloatPreflightReceipt {
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
) -> Result<(), StagingFloatPreflightError> {
    let document = &package.package().package().document;
    if let Some(footnote) = document.footnotes.first() {
        return Err(StagingFloatPreflightError::UnsupportedContent(
            footnote.node_id,
        ));
    }
    validate_blocks(package, &document.blocks, FloatBlockContext::DocumentBody)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FloatBlockContext {
    DocumentBody,
    ListItem,
    Caption,
}

fn validate_blocks(
    package: &ValidatedStagingAdvancedPackage,
    blocks: &[Block],
    context: FloatBlockContext,
) -> Result<(), StagingFloatPreflightError> {
    for block in blocks {
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                validate_body_inlines(children)?;
            }
            Block::List { node_id, items, .. } => {
                if context == FloatBlockContext::Caption {
                    return Err(StagingFloatPreflightError::UnsupportedContent(*node_id));
                }
                for item in items {
                    validate_blocks(package, &item.blocks, FloatBlockContext::ListItem)?;
                }
            }
            Block::Figure {
                node_id, caption, ..
            } => {
                let placement = package.figure_placement(*node_id);
                if context == FloatBlockContext::Caption {
                    return if placement == Some(FigurePlacement::Float) {
                        Err(StagingFloatPreflightError::NestedFloat(*node_id))
                    } else {
                        Err(StagingFloatPreflightError::UnsupportedContent(*node_id))
                    };
                }
                if placement == Some(FigurePlacement::Float)
                    && context != FloatBlockContext::DocumentBody
                {
                    return Err(StagingFloatPreflightError::NestedFloat(*node_id));
                }
                let computed = package
                    .package()
                    .package()
                    .style_sheet
                    .cascade_basic_document_style(
                        BasicStyleBlockKind::Figure,
                        block.classes(),
                        None,
                    )
                    .map_err(|_| StagingFloatPreflightError::UnsupportedStyle)?;
                if matches!(computed.width(), MachineFigureWidth::Auto) || computed.keep_with_next()
                {
                    return Err(StagingFloatPreflightError::UnsupportedStyle);
                }
                if placement == Some(FigurePlacement::Float)
                    && (computed.space_before().get().raw() != 0
                        || computed.space_after().get().raw() != 0
                        || computed.start_indent().get().raw() != 0
                        || computed.end_indent().get().raw() != 0
                        || computed.text_align().as_str() != "start"
                        || !computed.keep_caption())
                {
                    return Err(StagingFloatPreflightError::UnsupportedStyle);
                }
                validate_blocks(package, caption, FloatBlockContext::Caption)?;
            }
            Block::PageBreak { node_id, .. } => {
                if context == FloatBlockContext::Caption {
                    return Err(StagingFloatPreflightError::UnsupportedContent(*node_id));
                }
            }
            Block::Table { node_id, .. } => {
                return Err(StagingFloatPreflightError::UnsupportedContent(*node_id));
            }
        }
    }
    Ok(())
}

fn validate_body_inlines(inlines: &[Inline]) -> Result<(), StagingFloatPreflightError> {
    let mut stack: Vec<(&Inline, bool)> =
        inlines.iter().rev().map(|inline| (inline, false)).collect();
    while let Some((inline, inside_link)) = stack.pop() {
        match inline {
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
            Inline::Reference {
                node_id, format, ..
            } => {
                if *format != ReferenceFormat::Page {
                    return Err(StagingFloatPreflightError::UnsupportedContent(*node_id));
                }
            }
            Inline::Link {
                node_id, children, ..
            } => {
                if inside_link || !inlines_have_text(children) {
                    return Err(StagingFloatPreflightError::UnsupportedContent(*node_id));
                }
                stack.extend(children.iter().rev().map(|child| (child, true)));
            }
            Inline::Emphasis { node_id, .. }
            | Inline::Strong { node_id, .. }
            | Inline::FootnoteReference { node_id, .. } => {
                return Err(StagingFloatPreflightError::UnsupportedContent(*node_id));
            }
        }
    }
    Ok(())
}

fn validate_style_domain(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingFloatPreflightError> {
    package
        .package()
        .package()
        .style_sheet
        .validate_basic_document_styles()
        .map_err(|_| StagingFloatPreflightError::UnsupportedStyle)?;
    for rule in &package.package().package().style_sheet.rules {
        for declaration in &rule.declarations {
            if declaration.name == "page"
                && !matches!(&declaration.value, StyleValue::Keyword(value) if value == "auto")
            {
                return Err(StagingFloatPreflightError::UnsupportedStyle);
            }
        }
    }
    Ok(())
}

fn validate_master_form(
    package: &ValidatedStagingAdvancedPackage,
) -> Result<(), StagingFloatPreflightError> {
    let base = &package.package().package().page_masters;
    let advanced = package.page_masters();
    if base.masters.len() != 1
        || !base.selection_rules.is_empty()
        || base.default_master_id != base.masters[0].master_id
        || advanced.masters.len() != 1
    {
        return Err(StagingFloatPreflightError::UnsupportedMaster(None));
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
        return Err(StagingFloatPreflightError::UnsupportedMaster(Some(
            master.master_id.clone(),
        )));
    }
    validate_geometry(
        master.width.get().raw(),
        master.height.get().raw(),
        extension.trim,
    )
    .and_then(|()| validate_column_form(master.body, extension.column_layout.as_ref()))
    .map_err(|_| StagingFloatPreflightError::InvalidGeometry(master.master_id.clone()))
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
    if layout.fill != ColumnFill::Sequential || layout.balance != ColumnBalance::None {
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
    push_jcs_string(&mut output, FLOAT_PROFILE_RECEIPT_ALGORITHM);
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
    output.push_str(
        ",\"placement_classes\":[\"here\",\"top\",\"bottom\",\"next_page\"],\"profile\":",
    );
    push_jcs_string(&mut output, STAGING_FLOAT_PROFILE_ID);
    output.push_str(",\"style_sha256\":");
    push_hex(
        &mut output,
        package.package().epoch_identity().style().bytes(),
    );
    output.push_str(",\"text_wrap\":false}");
    output
}

fn push_limits(output: &mut String, limits: &ValidatedResourceLimits) {
    let limits = limits.get();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_candidate_class_order_is_contract_order() {
        assert_eq!(
            StagingFloatProfileDescriptor
                .placement_classes()
                .map(|class| class.as_str()),
            ["here", "top", "bottom", "next_page"]
        );
        assert!(!StagingFloatProfileDescriptor.supports_text_wrap());
        assert!(!StagingFloatProfileDescriptor.supports_column_balance());
    }
}
