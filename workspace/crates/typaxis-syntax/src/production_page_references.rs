//! Candidate labels live in a generated namespace separate from authored text.
use super::*;

pub(super) fn page_reference_key(owner: NodeId) -> typaxis_core::GeneratedBufferKey {
    typaxis_core::GeneratedBufferKey::new(owner, typaxis_core::GenerationKind::PageReference, 0)
}

/// Shared source validation. Legacy collection and observed copies own their
/// allocation/search operations; a failing observed search stops immediately.
pub(super) trait PageReferenceSink {
    type Error;
    type Retained;
    fn source_error(error: ProductionFlowError) -> Self::Error;
    // Original-source observation pays the paragraph visit independently of
    // its sites. Candidate-copy sinks retain their existing work contract.
    fn paragraph(&mut self, _owner: NodeId) -> Result<(), Self::Error> {
        Ok(())
    }
    fn step(&mut self, owner: NodeId) -> Result<(), Self::Error>;
    fn find(&mut self, values: &[(NodeId, u32)], owner: NodeId) -> Result<usize, Self::Error>;
    fn record_count(&self) -> usize;
    fn append(&mut self, owner: NodeId, page: u32, bytes: u64) -> Result<(), Self::Error>;
    fn retain(&mut self, values: &[(NodeId, u32)]) -> Result<Self::Retained, Self::Error>;
}

pub(super) fn page_reference_values_kernel<'a, S: PageReferenceSink>(
    paragraphs: &[ProductionTextParagraph<'a>],
    node_charge: u64,
    retained_text_bytes: u64,
    generated_bytes: &mut u64,
    values: Option<&[(NodeId, u32)]>,
    limits: &ValidatedResourceLimits,
    sink: &mut S,
) -> Result<Option<S::Retained>, S::Error> {
    let Some(values) = values else {
        return Ok(None);
    };
    let root = NodeId::new(0);
    let mismatch =
        |owner| S::source_error(failure(ProductionFlowErrorKind::ReceiptMismatch, owner));
    for pair in values.windows(2) {
        sink.step(root)?;
        if pair[0].0 >= pair[1].0 {
            return Err(mismatch(root));
        }
    }
    if node_charge
        .checked_add(values.len() as u64)
        .map_or(true, |n| n > limits.get().max_fragments)
    {
        return Err(S::source_error(failure(
            ProductionFlowErrorKind::NodeLimit,
            root,
        )));
    }
    let mut seen = 0usize;
    for paragraph in paragraphs {
        sink.paragraph(paragraph.owner())?;
        for site in paragraph.items() {
            sink.step(site.owner())?;
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
            let index = sink.find(values, owner)?;
            let page = values[index].1;
            if page == 0 || page > limits.get().max_pages {
                return Err(mismatch(owner));
            }
            seen = seen.checked_add(1).ok_or_else(|| mismatch(owner))?;
            let bytes = u64::from(page.ilog10()) + 1;
            *generated_bytes = generated_bytes
                .checked_add(bytes)
                .filter(|&total| {
                    bytes <= u64::from(limits.get().max_text_buffer_bytes)
                        && retained_text_bytes
                            .checked_add(total)
                            .is_some_and(|n| n <= limits.get().max_text_bytes)
                })
                .ok_or_else(|| {
                    S::source_error(failure(ProductionFlowErrorKind::TextLimit, owner))
                })?;
            if sink.record_count() as u64 >= limits.get().max_fragments {
                return Err(S::source_error(failure(
                    ProductionFlowErrorKind::NodeLimit,
                    owner,
                )));
            }
            sink.append(owner, page, bytes)?;
        }
    }
    if seen != values.len() {
        return Err(mismatch(root));
    }
    sink.retain(values).map(Some)
}

struct LegacyPageReferenceSink<'b, 'a, S: FlowSource<'a>> {
    source: &'b mut S,
    records: &'b mut Vec<(typaxis_core::GeneratedBufferKey, String)>,
    lifetime: std::marker::PhantomData<&'a ()>,
}
impl<'a, S: FlowSource<'a>> PageReferenceSink for LegacyPageReferenceSink<'_, 'a, S> {
    type Error = ProductionFlowError;
    type Retained = Vec<(NodeId, u32)>;
    fn source_error(error: ProductionFlowError) -> Self::Error {
        error
    }
    fn paragraph(&mut self, owner: NodeId) -> Result<(), Self::Error> {
        self.source.operations().step(owner, 1)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), Self::Error> {
        self.source.operations().step(owner, 1)
    }
    fn find(&mut self, values: &[(NodeId, u32)], owner: NodeId) -> Result<usize, Self::Error> {
        self.source
            .operations()
            .find(owner, values, |(id, _)| id.cmp(&owner), |_| 0)?
            .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))
    }
    fn record_count(&self) -> usize {
        self.records.len()
    }
    fn append(&mut self, owner: NodeId, page: u32, _: u64) -> Result<(), Self::Error> {
        self.source.reserve_records(owner, 1)?;
        self.source
            .operations()
            .grow(owner, self.records, 1, false)?;
        let text = self.source.operations().decimal(owner, page, false)?;
        self.records.push((page_reference_key(owner), text));
        Ok(())
    }
    fn retain(&mut self, values: &[(NodeId, u32)]) -> Result<Self::Retained, Self::Error> {
        let root = NodeId::new(0);
        let mut retained = Vec::new();
        self.source.reserve_records(root, values.len() as u64)?;
        self.source
            .operations()
            .grow(root, &mut retained, values.len(), true)?;
        self.source.operations().step(root, values.len() as u64)?;
        retained.extend_from_slice(values);
        Ok(retained)
    }
}

pub(super) fn add_page_reference_values<'a, S: FlowSource<'a>>(
    collector: &mut Collector<'a, S>,
    values: Option<&[(NodeId, u32)]>,
    limits: &ValidatedResourceLimits,
) -> Result<Option<Vec<(NodeId, u32)>>, ProductionFlowError> {
    page_reference_values_kernel(
        &collector.paragraphs,
        collector.node_charge,
        collector.retained_text_bytes,
        &mut collector.generated_bytes,
        values,
        limits,
        &mut LegacyPageReferenceSink {
            source: &mut collector.source,
            records: &mut collector.generated_records,
            lifetime: std::marker::PhantomData,
        },
    )
}
