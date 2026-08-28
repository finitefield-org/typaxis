use std::collections::BTreeMap;

use typaxis_core::{push_jcs_string, sha256, AnchorId, ImageResourceId, NodeId, SafeUri, TextSpan};
use typaxis_document::{Block, Inline, LinkTarget, PageRegionBlock, PageRegionInline};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub const ADVANCED_CONTENT_BINDING_ALGORITHM: &str =
    "typaxis.advanced-pagination-content-binding/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAdvancedImageUse {
    node_id: NodeId,
    image_id: ImageResourceId,
}

impl StagingAdvancedImageUse {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAdvancedAnchorUse {
    node_id: NodeId,
    anchor_id: AnchorId,
}

impl StagingAdvancedAnchorUse {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingAdvancedLinkTarget {
    Internal {
        anchor_id: AnchorId,
        anchor_owner: NodeId,
    },
    Uri(SafeUri),
}

impl StagingAdvancedLinkTarget {
    pub const fn internal_anchor(&self) -> Option<(&AnchorId, NodeId)> {
        match self {
            Self::Internal {
                anchor_id,
                anchor_owner,
            } => Some((anchor_id, *anchor_owner)),
            Self::Uri(_) => None,
        }
    }

    pub const fn uri(&self) -> Option<&SafeUri> {
        match self {
            Self::Internal { .. } => None,
            Self::Uri(uri) => Some(uri),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAdvancedLinkUse {
    node_id: NodeId,
    target: StagingAdvancedLinkTarget,
}

impl StagingAdvancedLinkUse {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn target(&self) -> &StagingAdvancedLinkTarget {
        &self.target
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAdvancedPageContent {
    page_index: u32,
    node_ids: Vec<NodeId>,
    images: Vec<StagingAdvancedImageUse>,
    anchors: Vec<StagingAdvancedAnchorUse>,
    links: Vec<StagingAdvancedLinkUse>,
    text: String,
}

impl StagingAdvancedPageContent {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub fn node_ids(&self) -> &[NodeId] {
        &self.node_ids
    }

    pub fn images(&self) -> &[StagingAdvancedImageUse] {
        &self.images
    }

    pub fn anchors(&self) -> &[StagingAdvancedAnchorUse] {
        &self.anchors
    }

    pub fn links(&self) -> &[StagingAdvancedLinkUse] {
        &self.links
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAdvancedContentBinding {
    resource_ledger_sha256: [u8; 32],
    pages: Vec<StagingAdvancedPageContent>,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}

impl StagingAdvancedContentBinding {
    pub const fn resource_ledger_sha256(&self) -> [u8; 32] {
        self.resource_ledger_sha256
    }

    pub fn pages(&self) -> &[StagingAdvancedPageContent] {
        &self.pages
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub fn page(&self, page_index: u32) -> Option<&StagingAdvancedPageContent> {
        self.pages.iter().find(|page| page.page_index == page_index)
    }

    pub fn verify(&self, page_count: usize) -> Result<(), StagingAdvancedContentError> {
        if self.pages.len() != page_count
            || self
                .pages
                .iter()
                .enumerate()
                .any(|(index, page)| u32::try_from(index) != Ok(page.page_index))
        {
            return Err(StagingAdvancedContentError::PageClosure);
        }
        let mut anchors = BTreeMap::new();
        for anchor in self.pages.iter().flat_map(|page| page.anchors()) {
            if anchors
                .insert(anchor.anchor_id().clone(), anchor.node_id())
                .is_some()
            {
                return Err(StagingAdvancedContentError::AnchorClosure);
            }
        }
        if self.pages.iter().flat_map(|page| page.links()).any(|link| {
            link.target()
                .internal_anchor()
                .is_some_and(|(anchor_id, owner)| anchors.get(anchor_id) != Some(&owner))
        }) {
            return Err(StagingAdvancedContentError::LinkClosure);
        }
        let canonical = encode_content(self.resource_ledger_sha256, &self.pages);
        if canonical != self.canonical_jcs || sha256(canonical.as_bytes()) != self.fingerprint {
            return Err(StagingAdvancedContentError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingAdvancedContentError {
    MissingNode,
    InvalidTextSpan,
    PageClosure,
    AnchorClosure,
    LinkClosure,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingAdvancedContentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MissingNode => "I9190: advanced content node is missing",
            Self::InvalidTextSpan => "I9190: advanced content text span is invalid",
            Self::PageClosure => "I9190: advanced content page closure mismatch",
            Self::AnchorClosure => "I9190: advanced content anchor closure mismatch",
            Self::LinkClosure => "I9190: advanced content link closure mismatch",
            Self::ReceiptMismatch => "I9190: advanced content receipt mismatch",
            Self::AllocationFailure => "L5110: advanced content allocation failure",
        })
    }
}

impl std::error::Error for StagingAdvancedContentError {}

pub(crate) fn bind_advanced_content(
    package: &ValidatedStagingAdvancedPackage,
    resource_ledger_sha256: [u8; 32],
    page_nodes: Vec<Vec<NodeId>>,
) -> Result<StagingAdvancedContentBinding, StagingAdvancedContentError> {
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(page_nodes.len())
        .map_err(|_| StagingAdvancedContentError::AllocationFailure)?;
    for (page_index, node_ids) in page_nodes.into_iter().enumerate() {
        let mut content = PageContentCollector::default();
        for node_id in &node_ids {
            if !append_node_content(package, *node_id, &mut content)? {
                return Err(StagingAdvancedContentError::MissingNode);
            }
        }
        let text = normalize_extracted_text(&content.text);
        pages.push(StagingAdvancedPageContent {
            page_index: u32::try_from(page_index)
                .map_err(|_| StagingAdvancedContentError::PageClosure)?,
            node_ids,
            images: content.images,
            anchors: content.anchors,
            links: content.links,
            text,
        });
    }
    let canonical_jcs = encode_content(resource_ledger_sha256, &pages);
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let binding = StagingAdvancedContentBinding {
        resource_ledger_sha256,
        pages,
        fingerprint,
        canonical_jcs,
    };
    binding.verify(binding.pages.len())?;
    Ok(binding)
}

#[derive(Default)]
struct PageContentCollector {
    text: String,
    images: Vec<StagingAdvancedImageUse>,
    anchors: Vec<StagingAdvancedAnchorUse>,
    links: Vec<StagingAdvancedLinkUse>,
}

fn append_node_content(
    package: &ValidatedStagingAdvancedPackage,
    node_id: NodeId,
    content: &mut PageContentCollector,
) -> Result<bool, StagingAdvancedContentError> {
    for block in &package.package().package().document.blocks {
        if append_matching_block(package, block, node_id, content)? {
            return Ok(true);
        }
    }
    for master in &package.page_masters().masters {
        for region in [
            master.header_content.as_ref(),
            master.footer_content.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for block in &region.blocks {
                if block.node_id() == node_id {
                    append_region_block(package, block, &mut content.text)?;
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn append_matching_block(
    package: &ValidatedStagingAdvancedPackage,
    block: &Block,
    target: NodeId,
    content: &mut PageContentCollector,
) -> Result<bool, StagingAdvancedContentError> {
    match block {
        Block::Paragraph {
            node_id, children, ..
        } => {
            if *node_id == target {
                append_inlines(package, children, content)?;
                return Ok(true);
            }
        }
        Block::Heading {
            node_id,
            anchor_id,
            children,
            ..
        } => {
            if *node_id == target {
                if let Some(anchor_id) = anchor_id {
                    content.anchors.push(StagingAdvancedAnchorUse {
                        node_id: *node_id,
                        anchor_id: anchor_id.clone(),
                    });
                }
                append_inlines(package, children, content)?;
                return Ok(true);
            }
        }
        Block::List { node_id, items, .. } => {
            if *node_id == target {
                for item in items {
                    for child in &item.blocks {
                        append_complete_block(package, child, content)?;
                    }
                }
                return Ok(true);
            }
            for item in items {
                if item.node_id == target {
                    for child in &item.blocks {
                        append_complete_block(package, child, content)?;
                    }
                    return Ok(true);
                }
                for child in &item.blocks {
                    if append_matching_block(package, child, target, content)? {
                        return Ok(true);
                    }
                }
            }
        }
        Block::Figure {
            node_id,
            image_id,
            caption,
            ..
        } => {
            if *node_id == target {
                content.images.push(StagingAdvancedImageUse {
                    node_id: *node_id,
                    image_id: *image_id,
                });
                for child in caption {
                    append_complete_block(package, child, content)?;
                }
                return Ok(true);
            }
            for child in caption {
                if append_matching_block(package, child, target, content)? {
                    return Ok(true);
                }
            }
        }
        Block::PageBreak { node_id, .. } => return Ok(*node_id == target),
        Block::Table { .. } => {}
    }
    Ok(false)
}

fn append_complete_block(
    package: &ValidatedStagingAdvancedPackage,
    block: &Block,
    content: &mut PageContentCollector,
) -> Result<(), StagingAdvancedContentError> {
    let node_id = match block {
        Block::Paragraph { node_id, .. }
        | Block::Heading { node_id, .. }
        | Block::List { node_id, .. }
        | Block::Table { node_id, .. }
        | Block::Figure { node_id, .. }
        | Block::PageBreak { node_id, .. } => *node_id,
    };
    if append_matching_block(package, block, node_id, content)? {
        Ok(())
    } else {
        Err(StagingAdvancedContentError::MissingNode)
    }
}

fn append_inlines(
    package: &ValidatedStagingAdvancedPackage,
    inlines: &[Inline],
    content: &mut PageContentCollector,
) -> Result<(), StagingAdvancedContentError> {
    for inline in inlines {
        match inline {
            Inline::Text { text_span, .. } => {
                append_text_span(package, *text_span, &mut content.text)?
            }
            Inline::Emphasis { children, .. } | Inline::Strong { children, .. } => {
                append_inlines(package, children, content)?
            }
            Inline::Link {
                node_id,
                target,
                children,
                ..
            } => {
                let target = match target {
                    LinkTarget::Internal(anchor_id) => {
                        let anchor_owner = package
                            .package()
                            .document_nodes()
                            .anchor_owner(anchor_id)
                            .ok_or(StagingAdvancedContentError::LinkClosure)?;
                        StagingAdvancedLinkTarget::Internal {
                            anchor_id: anchor_id.clone(),
                            anchor_owner,
                        }
                    }
                    LinkTarget::Uri(uri) => StagingAdvancedLinkTarget::Uri(uri.clone()),
                };
                content.links.push(StagingAdvancedLinkUse {
                    node_id: *node_id,
                    target,
                });
                append_inlines(package, children, content)?;
            }
            Inline::SoftBreak { .. } | Inline::HardBreak { .. } => content.text.push(' '),
            Inline::Anchor {
                node_id, anchor_id, ..
            } => content.anchors.push(StagingAdvancedAnchorUse {
                node_id: *node_id,
                anchor_id: anchor_id.clone(),
            }),
            Inline::Reference { .. } | Inline::FootnoteReference { .. } => {}
        }
    }
    content.text.push(' ');
    Ok(())
}

fn append_region_block(
    package: &ValidatedStagingAdvancedPackage,
    block: &PageRegionBlock,
    output: &mut String,
) -> Result<(), StagingAdvancedContentError> {
    for inline in block.children() {
        match inline {
            PageRegionInline::Text { text_span, .. } => {
                append_text_span(package, *text_span, output)?
            }
            PageRegionInline::SoftBreak { .. } | PageRegionInline::HardBreak { .. } => {
                output.push(' ')
            }
        }
    }
    output.push(' ');
    Ok(())
}

fn append_text_span(
    package: &ValidatedStagingAdvancedPackage,
    span: TextSpan,
    output: &mut String,
) -> Result<(), StagingAdvancedContentError> {
    let buffer = package
        .package()
        .package()
        .text_store
        .get(span.text_id())
        .ok_or(StagingAdvancedContentError::InvalidTextSpan)?;
    let start = usize::try_from(span.start_byte().get())
        .map_err(|_| StagingAdvancedContentError::InvalidTextSpan)?;
    let end = usize::try_from(span.end_byte().get())
        .map_err(|_| StagingAdvancedContentError::InvalidTextSpan)?;
    let value = buffer
        .text()
        .get(start..end)
        .ok_or(StagingAdvancedContentError::InvalidTextSpan)?;
    output.push_str(value);
    Ok(())
}

fn normalize_extracted_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn encode_content(
    resource_ledger_sha256: [u8; 32],
    pages: &[StagingAdvancedPageContent],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_CONTENT_BINDING_ALGORITHM);
    output.push_str(",\"pages\":[");
    for (page_ordinal, page) in pages.iter().enumerate() {
        if page_ordinal > 0 {
            output.push(',');
        }
        output.push_str("{\"anchors\":[");
        for (index, anchor) in page.anchors.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"anchor_id\":");
            push_jcs_string(&mut output, anchor.anchor_id.as_str());
            output.push_str(",\"node_id\":");
            output.push_str(&anchor.node_id.get().to_string());
            output.push('}');
        }
        output.push_str("],\"images\":[");
        for (index, image) in page.images.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"image_id\":");
            output.push_str(&image.image_id.get().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&image.node_id.get().to_string());
            output.push('}');
        }
        output.push_str("],\"links\":[");
        for (index, link) in page.links.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"node_id\":");
            output.push_str(&link.node_id.get().to_string());
            output.push_str(",\"target\":");
            match &link.target {
                StagingAdvancedLinkTarget::Internal {
                    anchor_id,
                    anchor_owner,
                } => {
                    output.push_str("{\"anchor_id\":");
                    push_jcs_string(&mut output, anchor_id.as_str());
                    output.push_str(",\"anchor_owner\":");
                    output.push_str(&anchor_owner.get().to_string());
                    output.push_str(",\"kind\":\"internal\"}");
                }
                StagingAdvancedLinkTarget::Uri(uri) => {
                    output.push_str("{\"kind\":\"uri\",\"uri\":");
                    push_jcs_string(&mut output, uri.as_str());
                    output.push('}');
                }
            }
            output.push('}');
        }
        output.push_str("],\"node_ids\":[");
        for (index, node_id) in page.node_ids.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&node_id.get().to_string());
        }
        output.push_str("],\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"text\":");
        push_jcs_string(&mut output, &page.text);
        output.push('}');
    }
    output.push_str("],\"resource_ledger_sha256\":");
    push_hex(&mut output, resource_ledger_sha256);
    output.push('}');
    output
}

pub(crate) fn push_content_binding(output: &mut String, binding: &StagingAdvancedContentBinding) {
    output.push_str(binding.canonical_jcs());
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
