//! Check semantic source views against stable host source bytes, independently
//! of contract-specific success receipts. Invalid identity maps are not admitted.
use super::*;
use typaxis_document_package::{
    WireAdvancedPageMasterSet, WirePageRegionBlock, WirePageRegionInline,
};
use typaxis_machine_input::AdmittedMachineSource;

#[derive(Debug)]
pub enum SemanticSourceFailure {
    #[cfg(feature = "book-v2-staging")]
    ReceiptMismatch,
    #[cfg(feature = "book-v2-staging")]
    Body(StagingSemanticSyntaxError),
    TextMapping {
        text_id: u32,
        mapping: usize,
        reason: SemanticMappingFailure,
    },
    SourceSpan {
        node_id: u32,
        source_id: u32,
        start_byte: u32,
        end_byte: u32,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticMappingFailure {
    TextLimit,
    GapOverlapOrEmpty,
    TextBoundary,
    SourceBoundary,
    SourceKind,
    IdentityBytes,
}
pub(super) fn validate_sources<K: Copy>(
    buffers: &[WireStagingM4TextBuffer],
    document: &WireSemanticDocument<K>,
    page_masters: &WireAdvancedPageMasterSet,
    sources: &[AdmittedMachineSource],
    limits: &ValidatedResourceLimits,
) -> Result<(), SemanticSourceFailure> {
    validate_source_mappings(buffers, sources, limits)?;
    check_blocks(&document.blocks, sources)?;
    for footnote in &document.footnotes {
        check_span(footnote.node_id, footnote.span, sources)?;
        check_blocks(&footnote.blocks, sources)?;
    }
    for master in &page_masters.masters {
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
                check_span(node, convert(span), sources)?;
                for child in children {
                    if let WirePageRegionInline::Text { node_id, span, .. } = child {
                        check_span(*node_id, convert(*span), sources)?;
                    }
                }
            }
        }
    }
    Ok(())
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
) -> Result<(), SemanticSourceFailure> {
    if source_slice(sources, span).is_none() {
        return Err(SemanticSourceFailure::SourceSpan {
            node_id,
            source_id: span.source_id,
            start_byte: span.start_byte,
            end_byte: span.end_byte,
        });
    }
    Ok(())
}
fn validate_source_mappings(
    buffers: &[WireStagingM4TextBuffer],
    sources: &[AdmittedMachineSource],
    limits: &ValidatedResourceLimits,
) -> Result<(), SemanticSourceFailure> {
    use SemanticMappingFailure as E;
    for buffer in buffers {
        let failure = |mapping, reason| SemanticSourceFailure::TextMapping {
            text_id: buffer.text_id,
            mapping,
            reason,
        };
        if buffer.utf8.len() as u64 > u64::from(limits.get().max_text_buffer_bytes) {
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
fn check_blocks<K: Copy>(
    blocks: &[WireSemanticBlock<K>],
    sources: &[AdmittedMachineSource],
) -> Result<(), SemanticSourceFailure> {
    use WireSemanticBlock as B;
    for block in blocks {
        let span = match block {
            B::DescriptionList { span, .. } => *span,
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
            B::DescriptionList { items, .. } => {
                for item in items {
                    check_span(item.node_id, item.span, sources)?;
                    check_span(item.term.node_id, item.term.span, sources)?;
                    check_inlines(&item.term.children, sources)?;
                    check_blocks(&item.blocks, sources)?;
                }
            }
            B::List { items, .. } => {
                for item in items {
                    check_span(item.node_id, item.span, sources)?;
                    check_blocks(&item.blocks, sources)?;
                }
            }
            B::Table {
                caption,
                head,
                body,
                ..
            } => {
                if let Some(caption) = caption {
                    check_blocks(caption, sources)?;
                }
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
) -> Result<(), SemanticSourceFailure> {
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
#[path = "semantic_host_source_tests.rs"]
mod tests;

impl std::fmt::Display for SemanticSourceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "book-v2-staging")]
            Self::ReceiptMismatch => f.write_str("source admission receipt mismatch"),
            #[cfg(feature = "book-v2-staging")]
            Self::Body(error) => write!(f, "{error}"),
            Self::TextMapping { text_id, mapping, reason } => write!(f, "text buffer {text_id} mapping {mapping}: {reason:?}"),
            Self::SourceSpan { node_id, source_id, start_byte, end_byte } => write!(f, "node {node_id} source {source_id} span {start_byte}..{end_byte} is not an admitted UTF-8 range"),
        }
    }
}
