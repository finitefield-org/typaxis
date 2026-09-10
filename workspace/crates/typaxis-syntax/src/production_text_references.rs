//! Source-owned Text/Number-reference labels. Frozen flows opt out; page
//! candidates retain their independent namespace and convergence semantics.
use super::*;
pub(super) fn text_reference_key(owner: NodeId) -> typaxis_core::GeneratedBufferKey {
    typaxis_core::GeneratedBufferKey::new(owner, typaxis_core::GenerationKind::Counter, 0)
}
pub(super) fn add_source_text_references<'a, S: FlowSource<'a>>(
    collector: &mut Collector<'a, S>,
    limits: &ValidatedResourceLimits,
) -> Result<(), ProductionFlowError> {
    if !collector.source.source_text_references() {
        return Ok(());
    }
    for pi in 0..collector.paragraphs.len() {
        for si in 0..collector.paragraphs[pi].items().len() {
            let site = collector.paragraphs[pi].items()[si];
            let Some(ProductionInlineReference::Anchor {
                target,
                target_owner,
                format:
                    format @ (ProductionReferenceFormat::Text | ProductionReferenceFormat::Number),
            }) = site.reference()
            else {
                continue;
            };
            let owner = site.owner();
            let missing = || failure(ProductionFlowErrorKind::MissingReferenceLabel, owner);
            let explicit = match format {
                ProductionReferenceFormat::Number => collector.source.reference_number(target),
                _ => collector.source.reference_label(target_owner),
            };
            let heading = if format == ProductionReferenceFormat::Text
                && explicit.is_none()
                && collector.source.heading_label(target_owner)
            {
                Some(
                    collector
                        .paragraphs
                        .iter()
                        .find(|p| p.owner() == target_owner)
                        .ok_or_else(missing)?,
                )
            } else {
                None
            };
            if explicit.is_none() && heading.is_none() {
                return Err(missing());
            }
            let visit = |emit: &mut dyn FnMut(&str) -> Result<(), ProductionFlowError>| {
                if let Some(label) = explicit {
                    emit(label)?;
                } else if let Some(heading) = heading {
                    for i in heading.items() {
                        match i.content() {
                            ProductionInlineContent::Text { utf8, .. } => emit(utf8)?,
                            ProductionInlineContent::SoftBreak
                            | ProductionInlineContent::HardBreak => emit(" ")?,
                            ProductionInlineContent::BeginEmphasis
                            | ProductionInlineContent::BeginStrong
                            | ProductionInlineContent::BeginLink
                            | ProductionInlineContent::EndContainer
                            | ProductionInlineContent::Anchor => {}
                            _ => return Err(missing()),
                        }
                    }
                }
                Ok(())
            };
            let mut length = 0usize;
            visit(&mut |text| {
                length = length
                    .checked_add(text.len())
                    .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, owner))?;
                Ok(())
            })?;
            if length == 0 {
                return Err(missing());
            }
            let total = collector
                .generated_bytes
                .checked_add(length as u64)
                .filter(|n| {
                    length as u64 <= u64::from(limits.get().max_text_buffer_bytes)
                        && collector
                            .retained_text_bytes
                            .checked_add(*n)
                            .is_some_and(|all| all <= limits.get().max_text_bytes)
                })
                .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, owner))?;
            if collector
                .node_charge
                .checked_add(collector.generated_records.len() as u64)
                .and_then(|v| v.checked_add(1))
                .is_none_or(|v| v > limits.get().max_fragments)
            {
                return Err(failure(ProductionFlowErrorKind::NodeLimit, owner));
            }
            // Reserve and copy only after the complete generated length and
            // shared text budget are known; authored buffers remain untouched.
            let mut text = String::new();
            text.try_reserve_exact(length)
                .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
            visit(&mut |part| {
                text.push_str(part);
                Ok(())
            })?;
            collector
                .generated_records
                .try_reserve(1)
                .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
            collector.generated_bytes = total;
            collector
                .generated_records
                .push((text_reference_key(owner), text));
        }
    }
    Ok(())
}
