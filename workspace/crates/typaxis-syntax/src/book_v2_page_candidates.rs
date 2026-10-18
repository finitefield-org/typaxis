//! Fallible copies of an authentic original source graph for candidate Page
//! labels. This is source storage, never a layout or output authorization.
use super::super::production_page_references::{page_reference_values_kernel, PageReferenceSink};
use super::*;
use typaxis_core::GeneratedBufferKey;
use typaxis_style::book_v2::BookV2StyleCopyError;
use typaxis_text::GeneratedOverlayCountedError;

/// Accepted cumulative logical records, requested allocation bytes and work.
/// Requested bytes are not an allocator-capacity or physical heap measurement.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct BookV2PageCandidateObservation {
    records: u64,
    spool: u64,
    work: u64,
}
impl BookV2PageCandidateObservation {
    pub const fn record_charge(&self) -> u64 {
        self.records
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool
    }
    pub const fn work_steps(&self) -> u64 {
        self.work
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2PageCandidateError {
    Source(ProductionFlowError),
    SpoolLimit { owner: NodeId },
    WorkLimit { owner: NodeId },
}
impl From<ProductionFlowError> for BookV2PageCandidateError {
    fn from(value: ProductionFlowError) -> Self {
        Self::Source(value)
    }
}
impl std::fmt::Display for BookV2PageCandidateError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => error.fmt(out),
            Self::SpoolLimit { owner } => {
                write!(out, "page_candidate spool_limit: node {}", owner.get())
            }
            Self::WorkLimit { owner } => {
                write!(out, "page_candidate work_limit: node {}", owner.get())
            }
        }
    }
}
impl std::error::Error for BookV2PageCandidateError {}
type Result<T> = std::result::Result<T, BookV2PageCandidateError>;
fn source_error(kind: ProductionFlowErrorKind, owner: NodeId) -> BookV2PageCandidateError {
    failure(kind, owner).into()
}

// This permit is issued only after the complete intrinsic source reservation.
// Its unused ordinal/grid slots remain charged; it grants no source authority.
pub(in crate::semantic_container::production_flow) struct SourceRecordPermit {
    remaining: u64,
}
struct OverlayCarrierPermit {
    count: u64,
    consumed: bool,
}
pub(in crate::semantic_container::production_flow) struct CandidateCopy<'o> {
    observed: &'o mut BookV2PageCandidateObservation,
    maximum_records: u64,
    maximum_spool: u64,
    maximum_work: u64,
}
impl CandidateCopy<'_> {
    fn charge(&mut self, owner: NodeId, records: u64, spool: u64, work: u64) -> Result<()> {
        self.observed.records = self
            .observed
            .records
            .checked_add(records)
            .filter(|n| *n <= self.maximum_records)
            .ok_or_else(|| source_error(ProductionFlowErrorKind::NodeLimit, owner))?;
        self.observed.spool = self
            .observed
            .spool
            .checked_add(spool)
            .filter(|n| *n <= self.maximum_spool)
            .ok_or(BookV2PageCandidateError::SpoolLimit { owner })?;
        self.step(owner, work)
    }
    pub(in crate::semantic_container::production_flow) fn step(
        &mut self,
        owner: NodeId,
        work: u64,
    ) -> Result<()> {
        self.observed.work = self
            .observed
            .work
            .checked_add(work)
            .filter(|n| *n <= self.maximum_work)
            .ok_or(BookV2PageCandidateError::WorkLimit { owner })?;
        Ok(())
    }
    fn prepay(
        &mut self,
        records: u64,
        overlay: u64,
    ) -> Result<(SourceRecordPermit, OverlayCarrierPermit)> {
        self.charge(NodeId::new(0), records, 0, 0)?;
        let remaining = records.checked_sub(overlay).ok_or_else(|| {
            source_error(ProductionFlowErrorKind::ReceiptMismatch, NodeId::new(0))
        })?;
        Ok((
            SourceRecordPermit { remaining },
            OverlayCarrierPermit {
                count: overlay,
                consumed: overlay == 0,
            },
        ))
    }
    fn overlay_request(
        &mut self,
        owner: NodeId,
        records: u64,
        spool: u64,
        work: u64,
        permit: &mut OverlayCarrierPermit,
    ) -> Result<()> {
        if records != 0 {
            if permit.consumed || records != permit.count {
                return Err(source_error(
                    ProductionFlowErrorKind::ReceiptMismatch,
                    owner,
                ));
            }
            permit.consumed = true;
        }
        self.charge(owner, 0, spool, work)
    }
    fn covered_request(
        &mut self,
        owner: NodeId,
        records: u64,
        spool: u64,
        work: u64,
        permit: &mut SourceRecordPermit,
    ) -> Result<()> {
        permit.remaining = permit
            .remaining
            .checked_sub(records)
            .ok_or_else(|| source_error(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
        self.charge(owner, 0, spool, work)
    }
    pub(in crate::semantic_container::production_flow) fn vector<T>(
        &mut self,
        owner: NodeId,
        count: usize,
        permit: &mut SourceRecordPermit,
    ) -> Result<Vec<T>> {
        let bytes = (count as u64)
            .checked_mul(std::mem::size_of::<T>() as u64)
            .ok_or(BookV2PageCandidateError::SpoolLimit { owner })?;
        self.covered_request(owner, count as u64, bytes, 1, permit)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(count)
            .map_err(|_| source_error(ProductionFlowErrorKind::AllocationFailure, owner))?;
        Ok(output)
    }
    pub(in crate::semantic_container::production_flow) fn copy_slice<T: Copy>(
        &mut self,
        owner: NodeId,
        values: &[T],
        permit: &mut SourceRecordPermit,
    ) -> Result<Vec<T>> {
        let mut output = self.vector(owner, values.len(), permit)?;
        self.step(owner, values.len() as u64)?;
        output.extend_from_slice(values);
        Ok(output)
    }
    fn string(&mut self, owner: NodeId, value: &str) -> Result<String> {
        self.charge(owner, 0, value.len() as u64, 1)?;
        let mut output = String::new();
        output
            .try_reserve_exact(value.len())
            .map_err(|_| source_error(ProductionFlowErrorKind::AllocationFailure, owner))?;
        self.step(owner, value.len() as u64)?;
        output.push_str(value);
        Ok(output)
    }
    pub(in crate::semantic_container::production_flow) fn style(
        &mut self,
        owner: NodeId,
        value: &SemanticContainerInheritanceStyle,
    ) -> Result<SemanticContainerInheritanceStyle> {
        value
            .try_copy_book_v2_counted(&mut |r, s, w| self.charge(owner, r, s, w))
            .map_err(|error| match error {
                BookV2StyleCopyError::Budget(error) => error,
                BookV2StyleCopyError::Allocation => {
                    source_error(ProductionFlowErrorKind::AllocationFailure, owner)
                }
            })
    }
    pub(in crate::semantic_container::production_flow) fn page_name(
        &mut self,
        owner: NodeId,
        value: Option<&typaxis_core::PageName>,
    ) -> Result<Option<typaxis_core::PageName>> {
        value
            .map(|name| {
                let copied = self.string(owner, name.as_str())?;
                typaxis_core::PageName::new(copied)
                    .map_err(|_| source_error(ProductionFlowErrorKind::ReceiptMismatch, owner))
            })
            .transpose()
    }
}

struct CopyPageSink<'b, 'o> {
    copy: &'b mut CandidateCopy<'o>,
    permit: &'b mut SourceRecordPermit,
    records: &'b mut Vec<(GeneratedBufferKey, String)>,
}
impl PageReferenceSink for CopyPageSink<'_, '_> {
    type Error = BookV2PageCandidateError;
    type Retained = Vec<(NodeId, u32)>;
    fn source_error(error: ProductionFlowError) -> Self::Error {
        error.into()
    }
    fn step(&mut self, owner: NodeId) -> Result<()> {
        self.copy.step(owner, 1)
    }
    fn find(&mut self, values: &[(NodeId, u32)], owner: NodeId) -> Result<usize> {
        let mut low = 0;
        let mut high = values.len();
        while low < high {
            self.copy.step(owner, 1)?;
            let mid = low + (high - low) / 2;
            match values[mid].0.cmp(&owner) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => return Ok(mid),
            }
        }
        Err(source_error(
            ProductionFlowErrorKind::ReceiptMismatch,
            owner,
        ))
    }
    fn record_count(&self) -> usize {
        self.records.len()
    }
    fn append(&mut self, owner: NodeId, mut page: u32, bytes: u64) -> Result<()> {
        // Decimal digits are bounded by u32; no formatting heap is hidden here.
        let mut digits = [0u8; 10];
        let mut start = digits.len();
        loop {
            self.copy.step(owner, 1)?;
            start -= 1;
            digits[start] = b'0' + (page % 10) as u8;
            page /= 10;
            if page == 0 {
                break;
            }
        }
        if digits.len() - start != bytes as usize {
            return Err(source_error(
                ProductionFlowErrorKind::ReceiptMismatch,
                owner,
            ));
        }
        let utf8 = std::str::from_utf8(&digits[start..])
            .map_err(|_| source_error(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
        let text = self.copy.string(owner, utf8)?;
        self.copy.step(owner, 1)?;
        self.records.push((page_reference_key(owner), text));
        Ok(())
    }
    fn retain(&mut self, values: &[(NodeId, u32)]) -> Result<Self::Retained> {
        self.copy.copy_slice(NodeId::new(0), values, self.permit)
    }
}

fn sort_records(
    copy: &mut CandidateCopy<'_>,
    records: &mut [(GeneratedBufferKey, String)],
) -> Result<()> {
    fn sift(
        copy: &mut CandidateCopy<'_>,
        records: &mut [(GeneratedBufferKey, String)],
        mut root: usize,
        end: usize,
    ) -> Result<()> {
        loop {
            let Some(left) = root
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .filter(|n| *n < end)
            else {
                return Ok(());
            };
            let mut child = left;
            if left + 1 < end {
                copy.step(records[left].0.owner(), 1)?;
                if records[left].0 < records[left + 1].0 {
                    child += 1;
                }
            }
            copy.step(records[root].0.owner(), 1)?;
            if records[root].0 >= records[child].0 {
                return Ok(());
            }
            copy.step(records[root].0.owner(), 1)?;
            records.swap(root, child);
            root = child;
        }
    }
    for root in (0..records.len() / 2).rev() {
        sift(copy, records, root, records.len())?;
    }
    for end in (1..records.len()).rev() {
        copy.step(records[0].0.owner(), 1)?;
        records.swap(0, end);
        sift(copy, records, 0, end)?;
    }
    Ok(())
}

/// Copy an authentic unlabelled source, changing only its generated Page labels.
/// Every accepted prefix survives errors. The old constructors retain their own
/// reservation/failure order; this new path prepays intrinsic records atomically.
/// Initial style lowering and downstream full verification are separate stages.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_page_reference_candidate_counted<'a>(
    original: &PreparedBookV2TextFlow<'a>,
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: &[(NodeId, u32)],
    prior_records: u64,
    prior_spool: u64,
    prior_work: u64,
    maximum_records: u64,
    maximum_work: u64,
    observed: &mut BookV2PageCandidateObservation,
) -> Result<PreparedBookV2TextFlow<'a>> {
    *observed = BookV2PageCandidateObservation {
        records: prior_records,
        spool: prior_spool,
        work: prior_work,
    };
    let root = NodeId::new(0);
    if !std::ptr::eq(original.package, body)
        || !std::ptr::eq(original.navigation, navigation)
        || original.page_reference_values.is_some()
        || navigation.verify_for(body).is_err()
    {
        return Err(source_error(ProductionFlowErrorKind::ReceiptMismatch, root));
    }
    let limits = body.body().limits();
    let mut copy = CandidateCopy {
        observed,
        maximum_records: maximum_records.min(limits.get().max_fragments),
        maximum_spool: limits.get().max_spool_bytes,
        maximum_work,
    };
    copy.charge(root, 0, 0, 0)?;
    let mut nodes = 0u64;
    let mut pages = 0u64;
    for event in &original.events {
        copy.step(root, 1)?;
        if matches!(event, ProductionFlowEvent::Begin { .. }) {
            nodes = nodes
                .checked_add(1)
                .ok_or_else(|| source_error(ProductionFlowErrorKind::NodeLimit, root))?;
        }
    }
    for paragraph in &original.paragraphs {
        copy.step(paragraph.owner, 1)?;
        for site in &paragraph.items {
            copy.step(site.owner, 1)?;
            if !matches!(site.content, ProductionInlineContent::EndContainer) {
                nodes = nodes
                    .checked_add(1)
                    .ok_or_else(|| source_error(ProductionFlowErrorKind::NodeLimit, site.owner))?;
            }
            if matches!(
                site.reference,
                Some(ProductionInlineReference::Anchor {
                    format: ProductionReferenceFormat::Page,
                    ..
                })
            ) {
                pages = pages
                    .checked_add(1)
                    .ok_or_else(|| source_error(ProductionFlowErrorKind::NodeLimit, site.owner))?;
            }
        }
    }
    let intrinsic = pages
        .checked_mul(3)
        .and_then(|n| original.source_record_charge.checked_add(n))
        .ok_or_else(|| source_error(ProductionFlowErrorKind::NodeLimit, root))?;
    let count = original
        .generated
        .buffers()
        .len()
        .checked_add(
            usize::try_from(pages)
                .map_err(|_| source_error(ProductionFlowErrorKind::NodeLimit, root))?,
        )
        .ok_or_else(|| source_error(ProductionFlowErrorKind::NodeLimit, root))?;
    let (mut permit, mut overlay_permit) = copy.prepay(intrinsic, count as u64)?;
    let mut records = copy.vector(root, count, &mut permit)?;
    for buffer in original.generated.buffers() {
        let key = buffer.key();
        let text = copy.string(key.owner(), buffer.utf8())?;
        copy.step(key.owner(), 1)?;
        records.push((key, text));
    }
    let mut retained_text_bytes = navigation.retained_text_bytes();
    for (owner, name) in &original.named_page_breaks {
        copy.step(*owner, 1)?;
        retained_text_bytes = retained_text_bytes
            .checked_add(name.as_str().len() as u64)
            .filter(|n| *n <= limits.get().max_text_bytes)
            .ok_or_else(|| source_error(ProductionFlowErrorKind::TextLimit, *owner))?;
    }
    let mut generated_bytes = original.generated.generated_bytes();
    let page_reference_values = page_reference_values_kernel(
        &original.paragraphs,
        nodes,
        retained_text_bytes,
        &mut generated_bytes,
        Some(values),
        limits,
        &mut CopyPageSink {
            copy: &mut copy,
            permit: &mut permit,
            records: &mut records,
        },
    )?;
    sort_records(&mut copy, &mut records)?;
    let generated = typaxis_text::GeneratedTextOverlay::try_from_sorted_records_counted(
        records,
        limits,
        retained_text_bytes,
        &mut |r, s, w| copy.overlay_request(root, r, s, w, &mut overlay_permit),
    )
    .map_err(|error| match error {
        GeneratedOverlayCountedError::Budget(error) => error,
        GeneratedOverlayCountedError::Allocation => {
            source_error(ProductionFlowErrorKind::AllocationFailure, root)
        }
        GeneratedOverlayCountedError::Overlay(_) => {
            source_error(ProductionFlowErrorKind::TextLimit, root)
        }
    })?;
    if !overlay_permit.consumed {
        return Err(source_error(ProductionFlowErrorKind::ReceiptMismatch, root));
    }
    let events = copy.copy_slice(root, &original.events, &mut permit)?;
    let mut paragraphs = copy.vector(root, original.paragraphs.len(), &mut permit)?;
    for paragraph in &original.paragraphs {
        let owner = paragraph.owner;
        let style = copy.style(owner, &paragraph.style)?;
        let page_name = copy.page_name(owner, paragraph.page_name.as_ref())?;
        let items = copy.copy_slice(owner, &paragraph.items, &mut permit)?;
        copy.step(owner, 1)?;
        paragraphs.push(ProductionTextParagraph {
            owner,
            style,
            page_name,
            items,
        });
    }
    let mut figures = copy.vector(root, original.figures.len(), &mut permit)?;
    for figure in &original.figures {
        let style = copy.style(figure.owner, &figure.style)?;
        let page_name = copy.page_name(figure.owner, figure.page_name.as_ref())?;
        copy.step(figure.owner, 1)?;
        figures.push(ProductionFigure {
            style,
            page_name,
            owner: figure.owner,
            source_span: figure.source_span,
            image_id: figure.image_id,
            alternative: figure.alternative,
            placement: figure.placement,
        });
    }
    let mut named_page_breaks = copy.vector(root, original.named_page_breaks.len(), &mut permit)?;
    for (owner, name) in &original.named_page_breaks {
        let name = copy
            .page_name(*owner, Some(name))?
            .ok_or_else(|| source_error(ProductionFlowErrorKind::ReceiptMismatch, *owner))?;
        copy.step(*owner, 1)?;
        named_page_breaks.push((*owner, name));
    }
    let mut tables = copy.vector(root, original.tables.len(), &mut permit)?;
    for table in &original.tables {
        let table = table.try_copy_for_page_candidate(&mut copy, &mut permit)?;
        copy.step(table.owner(), 1)?;
        tables.push(table);
    }
    let mut lists = copy.vector(root, original.lists.len(), &mut permit)?;
    for list in &original.lists {
        let style = copy.style(list.owner, &list.style)?;
        let page_name = copy.page_name(list.owner, list.page_name.as_ref())?;
        copy.step(list.owner, 1)?;
        lists.push(ProductionList {
            owner: list.owner,
            ordered: list.ordered,
            start: list.start,
            style,
            page_name,
        });
    }
    let mut list_items = copy.vector(root, original.list_items.len(), &mut permit)?;
    for item in &original.list_items {
        copy.step(item.owner, 1)?;
        list_items.push(ProductionListItem {
            owner: item.owner,
            list_index: item.list_index,
            item_index: item.item_index,
            source_span: item.source_span,
            language: item.language,
            ordered_value: item.ordered_value,
        });
    }
    let mut description_lists = copy.vector(root, original.description_lists.len(), &mut permit)?;
    for list in &original.description_lists {
        let list = list.try_copy_for_page_candidate(&mut copy)?;
        copy.step(list.owner(), 1)?;
        description_lists.push(list);
    }
    let mut description_items = copy.vector(root, original.description_items.len(), &mut permit)?;
    for item in &original.description_items {
        copy.step(item.owner(), 1)?;
        description_items.push(item.copy_for_page_candidate());
    }
    let mut footnote_definitions =
        copy.vector(root, original.footnote_definitions.len(), &mut permit)?;
    for definition in &original.footnote_definitions {
        copy.step(definition.owner, 1)?;
        footnote_definitions.push(ProductionFootnoteDefinition {
            owner: definition.owner,
            id: definition.id,
            style_paragraph: definition.style_paragraph,
            language: definition.language,
        });
    }
    let footnote_base_style = original
        .footnote_base_style
        .as_ref()
        .map(|style| copy.style(root, style))
        .transpose()?;
    let mut flow = SourceTextFlow {
        package: body,
        navigation,
        events,
        paragraphs,
        figures,
        named_page_breaks,
        tables,
        table_record_charge: original.table_record_charge,
        source_record_charge: intrinsic,
        lists,
        list_items,
        description_lists,
        description_items,
        footnote_definitions,
        footnote_base_style,
        generated,
        page_reference_values,
        text_bytes: original.text_bytes,
        fingerprint: [0; 32],
    };
    let limits_hash = counted_hash(&mut copy, |out| write_book_v2_flow_limits(limits, out))?;
    let languages_hash = counted_hash(&mut copy, |out| {
        write_book_v2_flow_languages(navigation, out)
    })?;
    flow.fingerprint = counted_hash(&mut copy, |out| {
        super::super::canonical::write_source_flow(
            &flow,
            BOOK_V2_TEXT_FLOW_ALGORITHM,
            languages_hash,
            limits_hash,
            body.body().canonical_jcs_sha256(),
            out,
        )
    })?;
    Ok(flow)
}

struct CountedSha<'b, 'o> {
    copy: &'b mut CandidateCopy<'o>,
    hash: Sha256,
    failure: Option<BookV2PageCandidateError>,
}
impl std::fmt::Write for CountedSha<'_, '_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self.failure.is_some() {
            return Err(std::fmt::Error);
        }
        let work = (text.len() as u64)
            .checked_add((text.len() as u64).div_ceil(64))
            .and_then(|n| n.checked_add(1));
        if let Err(error) = self.copy.step(NodeId::new(0), work.unwrap_or(u64::MAX)) {
            self.failure = Some(error);
            return Err(std::fmt::Error);
        }
        self.hash.write_str(text)
    }
}
fn counted_hash(
    copy: &mut CandidateCopy<'_>,
    write: impl FnOnce(&mut CountedSha<'_, '_>) -> std::fmt::Result,
) -> Result<[u8; 32]> {
    let mut output = CountedSha {
        copy,
        hash: Sha256::new(),
        failure: None,
    };
    if write(&mut output).is_err() {
        return Err(output.failure.unwrap_or_else(|| {
            source_error(ProductionFlowErrorKind::ReceiptMismatch, NodeId::new(0))
        }));
    }
    output.copy.step(NodeId::new(0), 1)?;
    Ok(output.hash.finish())
}

#[cfg(test)]
#[path = "book_v2_page_candidate_tests.rs"]
mod tests;
