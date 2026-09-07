//! Candidate labels live in a generated namespace separate from authored text.
use super::*;

pub(super) fn page_reference_key(owner: NodeId) -> typaxis_core::GeneratedBufferKey {
    typaxis_core::GeneratedBufferKey::new(owner, typaxis_core::GenerationKind::PageReference, 0)
}

pub(super) fn add_page_reference_values(
    collector: &mut Collector<'_>,
    values: Option<&[(NodeId, u32)]>,
    limits: &M4EffectiveResourceLimits,
) -> Result<Option<Vec<(NodeId, u32)>>, ProductionFlowError> {
    let Some(values) = values else {
        return Ok(None);
    };
    let root = NodeId::new(0);
    let mismatch = |owner| failure(ProductionFlowErrorKind::ReceiptMismatch, owner);
    if values.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(mismatch(root));
    }
    if collector
        .node_charge
        .checked_add(values.len() as u64)
        .is_none_or(|n| n > limits.base().get().max_fragments)
    {
        return Err(failure(ProductionFlowErrorKind::NodeLimit, root));
    }
    let mut seen = 0usize;
    for site in collector.paragraphs.iter().flat_map(|p| p.items()) {
        if !matches!(
            site.reference(),
            Some(ProductionInlineReference::Anchor {
                format: ProductionReferenceFormat::Page,
                ..
            })
        ) {
            continue;
        }
        let owner = site.owner();
        let index = values
            .binary_search_by_key(&owner, |(id, _)| *id)
            .map_err(|_| mismatch(owner))?;
        let page = values[index].1;
        if page == 0 || page > limits.base().get().max_pages {
            return Err(mismatch(owner));
        }
        seen = seen.checked_add(1).ok_or_else(|| mismatch(owner))?;
        let bytes = u64::from(page.ilog10()) + 1;
        collector.generated_bytes = collector
            .generated_bytes
            .checked_add(bytes)
            .filter(|&total| {
                bytes <= u64::from(limits.base().get().max_text_buffer_bytes)
                    && collector
                        .retained_text_bytes
                        .checked_add(total)
                        .is_some_and(|n| n <= limits.base().get().max_text_bytes)
            })
            .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, owner))?;
        if collector.generated_records.len() as u64 >= limits.base().get().max_fragments {
            return Err(failure(ProductionFlowErrorKind::NodeLimit, owner));
        }
        collector
            .generated_records
            .try_reserve(1)
            .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
        collector
            .generated_records
            .push((page_reference_key(owner), page.to_string()));
    }
    if seen != values.len() {
        return Err(mismatch(root));
    }
    let mut retained = Vec::new();
    retained
        .try_reserve_exact(values.len())
        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, root))?;
    retained.extend_from_slice(values);
    Ok(Some(retained))
}
