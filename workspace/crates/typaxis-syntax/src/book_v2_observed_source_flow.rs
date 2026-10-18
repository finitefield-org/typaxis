//! Full source construction/recollection with requested-storage observations.
//! The source product is unchanged; neither counters nor hashes authorize a
//! foreign body/navigation or replace full reconstruction.
use super::super::source_ops::{
    BookV2ObservedSourceFlowError as Error, BookV2ObservedSourceFlowObservation as Observation,
};
use super::super::source_ops::{ObservedSourceState, SourceOps};
use super::*;

struct ObservedBookV2Source<'a, 's> {
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    state: &'s mut ObservedSourceState,
}
impl<'a> FlowSource<'a> for ObservedBookV2Source<'a, '_> {
    type Kind = typaxis_document_package::book_v2::WireBookV2SemanticContainerKind;
    fn limits(&self) -> &'a ValidatedResourceLimits {
        self.body.body().limits()
    }
    fn operations(&mut self) -> SourceOps<'_> {
        SourceOps::Observed(self.state)
    }
    fn container_inheritance(
        &mut self,
        owner: NodeId,
    ) -> Result<Option<&'a SemanticContainerInheritanceStyle>, ProductionFlowError> {
        self.body
            .container_style_with_work(owner, &mut |work| self.state.charge(owner, 0, 0, work))
            .map(|v| v.map(|s| s.inheritance_style()))
    }
    fn language(&mut self, owner: NodeId) -> Result<Option<&'a str>, ProductionFlowError> {
        let values = self.navigation.languages();
        let index = self
            .operations()
            .find(owner, values, |r| r.node_id().cmp(&owner), |_| 0)?;
        Ok(index.map(|i| values[i].effective_language()))
    }
    fn anchors(&self) -> &'a [(AnchorId, NodeId)] {
        self.navigation.anchors()
    }
    fn source_text_references(&self) -> bool {
        true
    }
    fn reference_number(
        &mut self,
        owner: NodeId,
        target: &str,
    ) -> Result<Option<&'a str>, ProductionFlowError> {
        let values = self.navigation.number_bindings();
        let index = self.operations().find(
            owner,
            values,
            |v| v.anchor_id().as_str().cmp(target),
            |v| (v.anchor_id().as_str().len() as u64).saturating_add(target.len() as u64),
        )?;
        Ok(index.map(|i| values[i].label()))
    }
    fn reference_label(
        &mut self,
        owner: NodeId,
        target: NodeId,
    ) -> Result<Option<&'a str>, ProductionFlowError> {
        for entry in self.navigation.outline() {
            self.state.charge(owner, 0, 0, 1)?;
            if entry.source.node_id == target {
                return Ok(Some(&entry.label));
            }
        }
        Ok(None)
    }
    fn heading_label(
        &mut self,
        owner: NodeId,
        target: NodeId,
    ) -> Result<bool, ProductionFlowError> {
        let values = self.navigation.languages();
        let i = self
            .operations()
            .find(owner, values, |v| v.node_id().cmp(&target), |_| 0)?;
        Ok(i.is_some_and(|i| {
            values[i].kind() == typaxis_document::book_v2::BookV2LanguageNodeKind::Heading
        }))
    }
    fn reserve_records(&mut self, owner: NodeId, count: u64) -> Result<(), ProductionFlowError> {
        self.state.logical(owner, count)
    }
    fn source_record_charge(&self) -> u64 {
        self.state.logical
    }
}

/// Full original source preparation under cumulative record/requested-byte/work
/// ceilings. Earlier history and every accepted prefix survive every Result.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_text_flow_observed_counted<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    prior_records: u64,
    prior_spool: u64,
    prior_work: u64,
    maximum_records: u64,
    maximum_work: u64,
    observed: &mut Observation,
) -> Result<PreparedBookV2TextFlow<'a>, Error> {
    prepare_observed(
        body,
        navigation,
        None,
        [prior_records, prior_spool, prior_work],
        maximum_records,
        maximum_work,
        observed,
    )
}

/// Recollect candidate values with the same complete source kernel. Values do
/// not prove final placement and cannot change original source ownership.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_text_flow_with_page_references_observed_counted<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: &[(NodeId, u32)],
    prior_records: u64,
    prior_spool: u64,
    prior_work: u64,
    maximum_records: u64,
    maximum_work: u64,
    observed: &mut Observation,
) -> Result<PreparedBookV2TextFlow<'a>, Error> {
    prepare_observed(
        body,
        navigation,
        Some(values),
        [prior_records, prior_spool, prior_work],
        maximum_records,
        maximum_work,
        observed,
    )
}

fn prepare_observed<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: Option<&[(NodeId, u32)]>,
    prior: [u64; 3],
    maximum_records: u64,
    maximum_work: u64,
    observed: &mut Observation,
) -> Result<PreparedBookV2TextFlow<'a>, Error> {
    let limits = body.body().limits();
    let mut state = ObservedSourceState::new(
        prior,
        [
            maximum_records.min(limits.get().max_fragments),
            limits.get().max_spool_bytes,
            maximum_work,
        ],
    );
    let result = prepare_kernel(body, navigation, values, &mut state);
    *observed = state.progress;
    result.map_err(|e| state.finish_error(e))
}

fn prepare_kernel<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: Option<&[(NodeId, u32)]>,
    state: &mut ObservedSourceState,
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    let root = NodeId::new(0);
    state.charge(root, 0, 0, 0)?;
    state.charge(root, 0, 0, 1)?;
    navigation
        .verify_for(body)
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let wire = body.body().wire();
    let limits = body.body().limits();
    let mut source = ObservedBookV2Source {
        body,
        navigation,
        state,
    };
    source.reserve_records(root, 1)?;
    let rules = source
        .operations()
        .lower(wire.style_sheet(), limits, true)?;
    let mut flow = collect_source_flow(
        source,
        body,
        navigation,
        wire.document(),
        wire.text_buffers(),
        rules,
        navigation.retained_text_bytes(),
        limits,
        values,
    )?;
    // Before the unchanged streaming writers format/escape their inputs, pay
    // their typed preparation traversal. Any i64/u64 spelling fits32ASCII
    // preparation units; every input byte has a six-byte JCS escape bound.
    state.charge(root, 0, 0, 5 * 32 + 1)?;
    let limits_sha = counted_hash(state, |w| write_book_v2_flow_limits(limits, w))?;
    state.escape_input(root, "typaxis.book-2-source-flow-languages/1")?;
    for record in navigation.languages() {
        state.charge(record.node_id(), 0, 0, 32 + 1)?;
        state.escape_input(record.node_id(), record.effective_language())?;
    }
    let languages_sha = counted_hash(state, |w| write_book_v2_flow_languages(navigation, w))?;
    prepare_canonical(state, &flow)?;
    flow.fingerprint = counted_hash(state, |w| {
        super::super::canonical::write_source_flow(
            &flow,
            BOOK_V2_TEXT_FLOW_ALGORITHM,
            languages_sha,
            limits_sha,
            body.body().canonical_jcs_sha256(),
            w,
        )
    })?;
    Ok(flow)
}

fn prepare_canonical(
    state: &mut ObservedSourceState,
    flow: &PreparedBookV2TextFlow<'_>,
) -> Result<(), ProductionFlowError> {
    let root = NodeId::new(0);
    state.escape_input(root, BOOK_V2_TEXT_FLOW_ALGORITHM)?;
    // Four32byte hashes become two fixed hexadecimal characters per byte.
    // Eight preparation units per byte cover the loop visit and both bounded
    // nibble/index/ASCII conversions; this is not machine-instruction counting.
    // constant writer literals themselves need no input-format/escape scan.
    state.charge(root, 0, 0, 4 * 32 * 8 + 32)?;
    for event in &flow.events {
        state.charge(root, 0, 0, 32 + 1)?;
        if let ProductionFlowEvent::Begin { kind, .. } = event {
            state.escape_input(root, kind.as_str())?;
        }
    }
    for (owner, name) in &flow.named_page_breaks {
        state.charge(*owner, 0, 0, 32 + 1)?;
        state.escape_input(*owner, name.as_str())?;
    }
    for definition in &flow.footnote_definitions {
        state.charge(definition.owner, 0, 0, 2 * 32 + 1)?;
        state.escape_input(definition.owner, definition.id)?;
        state.escape_input(definition.owner, definition.language)?;
    }
    for paragraph in &flow.paragraphs {
        let owner = paragraph.owner;
        state.charge(owner, 0, 0, 3 * 32 + 1)?;
        for family in paragraph.style.font_families().unwrap_or(&[]) {
            state.escape_input(owner, family)?;
        }
        for site in &paragraph.items {
            state.charge(site.owner, 0, 0, 32 + 1)?;
            state.escape_input(site.owner, site.content.as_str())?;
            state.escape_input(site.owner, site.language)?;
            if let ProductionInlineContent::Text { utf8, .. } = site.content {
                state.charge(site.owner, 0, 0, 3 * 32)?;
                state.escape_input(site.owner, utf8)?;
            }
        }
        if let Some(page) = &paragraph.page_name {
            state.escape_input(owner, page.as_str())?;
        }
    }
    Ok(())
}

struct CountedSha<'s> {
    state: &'s mut ObservedSourceState,
    hash: Sha256,
}
impl std::fmt::Write for CountedSha<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let n = text.len() as u64;
        let work = n
            .checked_add(n.div_ceil(64))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| {
                self.state.work_error(NodeId::new(0));
                std::fmt::Error
            })?;
        self.state
            .charge(NodeId::new(0), 0, 0, work)
            .map_err(|_| std::fmt::Error)?;
        std::fmt::Write::write_str(&mut self.hash, text)
    }
}
fn counted_hash(
    state: &mut ObservedSourceState,
    write: impl FnOnce(&mut CountedSha<'_>) -> std::fmt::Result,
) -> Result<[u8; 32], ProductionFlowError> {
    let mut output = CountedSha {
        state,
        hash: Sha256::new(),
    };
    write(&mut output).map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, NodeId::new(0)))?;
    output.state.charge(NodeId::new(0), 0, 0, 1)?;
    Ok(output.hash.finish())
}

impl<'a> PreparedBookV2TextFlow<'a> {
    /// Fully reconstruct every source field, then compare with paid typed
    /// equality. A matching fingerprint alone never authorizes this operation.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_for_observed_counted(
        &self,
        body: &StyledBookV2Body,
        navigation: &PreparedBookV2Navigation<'_>,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
        maximum_records: u64,
        maximum_work: u64,
        observed: &mut Observation,
    ) -> Result<(), Error> {
        let limits = self.body().body().limits();
        let mut state = ObservedSourceState::new(
            [prior_records, prior_spool, prior_work],
            [
                maximum_records.min(limits.get().max_fragments),
                limits.get().max_spool_bytes,
                maximum_work,
            ],
        );
        let result = (|| {
            state.charge(NodeId::new(0), 0, 0, 0)?;
            if !std::ptr::eq(self.body(), body) || !std::ptr::eq(self.navigation(), navigation) {
                return Err(failure(
                    ProductionFlowErrorKind::ReceiptMismatch,
                    NodeId::new(0),
                ));
            }
            let other = prepare_kernel(
                self.body(),
                self.navigation(),
                self.page_reference_values(),
                &mut state,
            )?;
            if !equal_flows(&mut state, self, &other)? {
                return Err(failure(
                    ProductionFlowErrorKind::ReceiptMismatch,
                    NodeId::new(0),
                ));
            }
            Ok(())
        })();
        *observed = state.progress;
        result.map_err(|e| state.finish_error(e))
    }
}

#[path = "book_v2_observed_source_equality.rs"]
mod equality;
use equality::equal_flows;

#[cfg(test)]
#[path = "book_v2_observed_source_tests.rs"]
mod tests;
