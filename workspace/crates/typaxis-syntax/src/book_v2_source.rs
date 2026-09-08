//! Stable host-source admission joined to the successor body and its exact text
//! maps. No resource bytes, profile or selected layout are admitted here.
use super::*;
use typaxis_document_package::book_v2::WireBookV2Block;
use typaxis_document_package::{WirePageRegionBlock, WirePageRegionInline};
use typaxis_machine_input::book_v2::{AdmittedBookV2Input, BookV2InputProvenance};
use typaxis_machine_input::{AdmittedMachineSource, MachineInputStage};

#[derive(Debug)]
pub enum BookV2SourceFailure {
    ReceiptMismatch,
    Body(StagingSemanticSyntaxError),
    TextMapping {
        text_id: u32,
        mapping: usize,
        reason: BookV2MappingFailure,
    },
    SourceSpan {
        node_id: u32,
        source_id: u32,
        start_byte: u32,
        end_byte: u32,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2MappingFailure {
    TextLimit,
    GapOverlapOrEmpty,
    TextBoundary,
    SourceBoundary,
    SourceKind,
    IdentityBytes,
}
#[derive(Debug)]
pub struct BookV2SourcePreparationError {
    failure: BookV2SourceFailure,
    provenance: Box<BookV2InputProvenance>,
}
impl BookV2SourcePreparationError {
    pub const fn failure(&self) -> &BookV2SourceFailure {
        &self.failure
    }
    pub fn provenance(&self) -> &BookV2InputProvenance {
        &self.provenance
    }
}
impl std::fmt::Display for BookV2SourcePreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 source preparation {:?}", self.failure)
    }
}
impl std::error::Error for BookV2SourcePreparationError {}

/// Source-admitted styled body. Pure carrier preparation cannot construct this
/// owner, and it cannot be passed to consumers requiring a legacy package.
///
/// ```compile_fail
/// use typaxis_syntax::{book_v2::SourceAdmittedBookV2Body, ValidatedProductionMachinePackage};
/// fn legacy(value: SourceAdmittedBookV2Body) -> ValidatedProductionMachinePackage { value }
/// ```
#[derive(Debug)]
pub struct SourceAdmittedBookV2Body {
    styled: StyledBookV2Body,
    sources: Vec<AdmittedMachineSource>,
    provenance: BookV2InputProvenance,
}
impl SourceAdmittedBookV2Body {
    pub const fn styled(&self) -> &StyledBookV2Body {
        &self.styled
    }
    pub fn sources(&self) -> &[AdmittedMachineSource] {
        &self.sources
    }
    pub const fn provenance(&self) -> &BookV2InputProvenance {
        &self.provenance
    }
}

pub fn prepare_admitted_book_v2_body(
    input: AdmittedBookV2Input,
    limits: &ValidatedResourceLimits,
) -> Result<SourceAdmittedBookV2Body, BookV2SourcePreparationError> {
    let (decoded, sources, provenance) = input.into_parts();
    let prepare = || -> Result<StyledBookV2Body, BookV2SourceFailure> {
        let progress = provenance.progress();
        if provenance.limits() != limits
            || decoded.limits() != limits
            || progress.stage() != MachineInputStage::SourcesAdmitted
            || progress.decoded_contract()
                != Some(typaxis_document_package::book_v2::BOOK_V2_DOCUMENT_PACKAGE_CONTRACT)
            || progress.canonical_sha256() != Some(decoded.canonical_jcs_sha256())
            || progress
                .package()
                .is_none_or(|p| p.sha256() != decoded.raw_sha256())
            || progress.fingerprint().is_none()
            || sources.len() != decoded.wire().sources().len()
            || sources.len() != progress.sources().len()
            || sources
                .iter()
                .zip(decoded.wire().sources())
                .zip(progress.sources())
                .any(|((source, declared), facts)| {
                    source.facts() != facts
                        || facts.source_id().get() != declared.source_id
                        || facts.uri().as_str() != declared.uri
                        || facts.bytes() != u64::from(declared.utf8_byte_length)
                        || source.text().len() as u64 != facts.bytes()
                        || hex_sha(facts.sha256()) != declared.sha256
                })
        {
            return Err(BookV2SourceFailure::ReceiptMismatch);
        }
        let body = prepare_book_v2_body(decoded, limits).map_err(BookV2SourceFailure::Body)?;
        validate_source_mappings(&body, &sources)?;
        check_blocks(&body.wire().document().blocks, &sources)?;
        for footnote in &body.wire().document().footnotes {
            check_span(footnote.node_id, footnote.span, &sources)?;
            check_blocks(&footnote.blocks, &sources)?;
        }
        for master in &body.wire().advanced_page_masters().masters {
            for region in [
                master.header_content.as_ref(),
                master.footer_content.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                for block in &region.blocks {
                    let (node, span, children) = match block {
                        WirePageRegionBlock::Paragraph {
                            node_id,
                            span,
                            children,
                            ..
                        }
                        | WirePageRegionBlock::Heading {
                            node_id,
                            span,
                            children,
                            ..
                        } => (*node_id, *span, children),
                    };
                    let convert =
                        |span: typaxis_document_package::WireSourceSpan| WireStagingSourceSpan {
                            source_id: span.source_id,
                            start_byte: span.start_byte,
                            end_byte: span.end_byte,
                        };
                    check_span(node, convert(span), &sources)?;
                    for child in children {
                        if let WirePageRegionInline::Text { node_id, span, .. } = child {
                            check_span(*node_id, convert(*span), &sources)?;
                        }
                    }
                }
            }
        }
        style_book_v2_body(body).map_err(BookV2SourceFailure::Body)
    };
    match prepare() {
        Ok(styled) => Ok(SourceAdmittedBookV2Body {
            styled,
            sources,
            provenance,
        }),
        Err(failure) => Err(BookV2SourcePreparationError {
            failure,
            provenance: Box::new(provenance),
        }),
    }
}
fn hex_sha(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn source_slice(sources: &[AdmittedMachineSource], span: WireStagingSourceSpan) -> Option<&str> {
    sources
        .get(span.source_id as usize)
        .filter(|source| source.facts().source_id().get() == span.source_id)?
        .text()
        .get(span.start_byte as usize..span.end_byte as usize)
}
fn check_span(
    node_id: u32,
    span: WireStagingSourceSpan,
    sources: &[AdmittedMachineSource],
) -> Result<(), BookV2SourceFailure> {
    if source_slice(sources, span).is_none() {
        return Err(BookV2SourceFailure::SourceSpan {
            node_id,
            source_id: span.source_id,
            start_byte: span.start_byte,
            end_byte: span.end_byte,
        });
    }
    Ok(())
}
fn validate_source_mappings(
    body: &PreparedBookV2Body,
    sources: &[AdmittedMachineSource],
) -> Result<(), BookV2SourceFailure> {
    use BookV2MappingFailure as E;
    for buffer in body.wire().text_buffers() {
        let failure = |mapping, reason| BookV2SourceFailure::TextMapping {
            text_id: buffer.text_id,
            mapping,
            reason,
        };
        if buffer.utf8.len() as u64 > u64::from(body.limits().get().max_text_buffer_bytes) {
            return Err(failure(0, E::TextLimit));
        }
        let mut cursor = 0u32;
        for (index, mapping) in buffer.mappings.iter().enumerate() {
            let range = mapping.text_range;
            if range.start_byte != cursor || range.start_byte >= range.end_byte {
                return Err(failure(index, E::GapOverlapOrEmpty));
            }
            let text = buffer
                .utf8
                .get(range.start_byte as usize..range.end_byte as usize)
                .ok_or_else(|| failure(index, E::TextBoundary))?;
            match (mapping.kind, mapping.source_span) {
                (WireStagingTextMapKind::Inserted, None) => {}
                (
                    WireStagingTextMapKind::Identity | WireStagingTextMapKind::Replacement,
                    Some(span),
                ) => {
                    let original = source_slice(sources, span)
                        .ok_or_else(|| failure(index, E::SourceBoundary))?;
                    if mapping.kind == WireStagingTextMapKind::Identity && original != text {
                        return Err(failure(index, E::IdentityBytes));
                    }
                }
                _ => return Err(failure(index, E::SourceKind)),
            }
            cursor = range.end_byte;
        }
        if cursor as usize != buffer.utf8.len() {
            return Err(failure(buffer.mappings.len(), E::GapOverlapOrEmpty));
        }
    }
    Ok(())
}
fn check_blocks(
    blocks: &[WireBookV2Block],
    sources: &[AdmittedMachineSource],
) -> Result<(), BookV2SourceFailure> {
    use WireBookV2Block as B;
    for block in blocks {
        let span = match block {
            B::Paragraph { span, .. }
            | B::Heading { span, .. }
            | B::List { span, .. }
            | B::Table { span, .. }
            | B::Figure { span, .. }
            | B::VectorFigure { span, .. }
            | B::SemanticContainer { span, .. }
            | B::DisplayMath { span, .. }
            | B::MathVectorBlock { span, .. }
            | B::PageBreak { span, .. } => *span,
        };
        check_span(block.node_id(), span, sources)?;
        match block {
            B::Paragraph { children, .. } | B::Heading { children, .. } => {
                check_inlines(children, sources)?
            }
            B::SemanticContainer { blocks, .. } => check_blocks(blocks, sources)?,
            B::List { items, .. } => {
                for item in items {
                    check_span(item.node_id, item.span, sources)?;
                    check_blocks(&item.blocks, sources)?;
                }
            }
            B::Table { head, body, .. } => {
                for row in head.iter().chain(body) {
                    check_span(row.node_id, row.span, sources)?;
                    for cell in &row.cells {
                        check_span(cell.node_id, cell.span, sources)?;
                        check_blocks(&cell.blocks, sources)?;
                    }
                }
            }
            B::Figure { caption, .. } | B::VectorFigure { caption, .. } => {
                check_blocks(caption, sources)?
            }
            B::MathVectorBlock {
                equation_number: Some(number),
                ..
            } => check_span(number.node_id, number.span, sources)?,
            _ => {}
        }
    }
    Ok(())
}
fn check_inlines(
    inlines: &[WireStagingM4Inline],
    sources: &[AdmittedMachineSource],
) -> Result<(), BookV2SourceFailure> {
    for inline in inlines {
        check_span(inline.node_id(), inline.span(), sources)?;
        match inline {
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } => check_inlines(children, sources)?,
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "book_v2_source_tests.rs"]
mod tests;
