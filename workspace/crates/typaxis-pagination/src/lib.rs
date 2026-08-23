#![forbid(unsafe_code)]

use core::fmt;
use core::num::NonZeroU16;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use typaxis_core::{
    initial_pagination_state_fingerprint_from_jcs,
    materialized_pagination_state_fingerprint_from_jcs, push_generated_buffer_key_jcs,
    push_jcs_string, AnchorId, FootnoteId, GeneratedBufferKey, GenerationKind,
    LayoutStateFingerprint, MasterId, NodeId, Point, PositiveLength, Rect, ReferenceFingerprint,
    Utf8ByteOffset, ValidatedResourceLimits, JSON_SAFE_INTEGER_MAX,
};
use typaxis_diagnostics::{AdvisoryDiagnostic, Diagnostic, DiagnosticCode, Severity};
use typaxis_document::GeneratedSiteTarget;
use typaxis_layout::{
    Continuation, DiscoveredAnchor, FlowCursor, FlowPosition, FlowTree, FragmentError,
    FragmentRequest, FragmentWorkBudget, Fragmenter, LayoutEpoch, PageContext,
};
use typaxis_style::PageMasterSet;
use typaxis_syntax::{PackagePaginationContext, ValidatedParsedPackage};
use typaxis_text::{GeneratedProvenance, GeneratedTextStore, GeneratedTextStoreError};

pub const FALLBACK_POLICY_ID: &str = "lowest_cost_then_earliest";

#[derive(Clone)]
struct PaginationSessionId(Arc<()>);

impl PaginationSessionId {
    fn issue() -> Self {
        Self(Arc::new(()))
    }

    fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for PaginationSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PaginationSessionId(<opaque>)")
    }
}

impl PartialEq for PaginationSessionId {
    fn eq(&self, other: &Self) -> bool {
        self.same_as(other)
    }
}

impl Eq for PaginationSessionId {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackPolicy {
    LowestCostThenEarliest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationOptions {
    max_pages: u32,
    max_layout_passes: u16,
    max_page_break_lookback: u16,
    max_fragments: u64,
    max_footnote_reflows_per_page: u16,
    max_column_balance_candidates: u16,
    max_float_queue: u32,
    max_float_carry_pages: u16,
    strict: bool,
}
impl PaginationOptions {
    pub fn from_limits(limits: &ValidatedResourceLimits, strict: bool) -> Self {
        let limits = limits.get();
        Self {
            max_pages: limits.max_pages,
            max_layout_passes: limits.max_layout_passes,
            max_page_break_lookback: limits.max_page_break_lookback,
            max_fragments: limits.max_fragments,
            max_footnote_reflows_per_page: limits.max_footnote_reflows_per_page,
            max_column_balance_candidates: limits.max_column_balance_candidates,
            max_float_queue: limits.max_float_queue,
            max_float_carry_pages: limits.max_float_carry_pages,
            strict,
        }
    }
    pub const fn fallback_policy(&self) -> FallbackPolicy {
        FallbackPolicy::LowestCostThenEarliest
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PaginationWorkBudget {
    session: PaginationSessionId,
    initial_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    next_pass_index: u16,
    max_pages: u32,
    remaining_layout_passes: u16,
    max_fragments: u64,
    max_page_break_lookback: u16,
    max_footnote_reflows_per_page: u16,
    max_column_balance_candidates: u16,
    max_float_queue: u32,
    max_float_carry_pages: u16,
    footnote_reflows: BTreeMap<(u16, u32), u16>,
    column_candidates: BTreeMap<(u16, NodeId), u16>,
    queued_floats: BTreeSet<(u16, NodeId, u32)>,
    dequeued_floats: BTreeSet<(u16, NodeId, u32)>,
    float_carries: BTreeMap<(u16, NodeId, u32), u16>,
    completed_passes: Vec<PassBudgetSummary>,
}
impl PaginationWorkBudget {
    fn new(
        session: PaginationSessionId,
        options: PaginationOptions,
        initial_fingerprint: LayoutStateFingerprint,
        layout_epoch: LayoutEpoch,
    ) -> Self {
        Self {
            session,
            initial_fingerprint,
            layout_epoch,
            next_pass_index: 0,
            max_pages: options.max_pages,
            remaining_layout_passes: options.max_layout_passes,
            max_fragments: options.max_fragments,
            max_page_break_lookback: options.max_page_break_lookback,
            max_footnote_reflows_per_page: options.max_footnote_reflows_per_page,
            max_column_balance_candidates: options.max_column_balance_candidates,
            max_float_queue: options.max_float_queue,
            max_float_carry_pages: options.max_float_carry_pages,
            footnote_reflows: BTreeMap::new(),
            column_candidates: BTreeMap::new(),
            queued_floats: BTreeSet::new(),
            dequeued_floats: BTreeSet::new(),
            float_carries: BTreeMap::new(),
            completed_passes: Vec::new(),
        }
    }
    fn consume_layout_pass(&mut self) -> Result<(), PaginationError> {
        self.remaining_layout_passes = self
            .remaining_layout_passes
            .checked_sub(1)
            .ok_or(PaginationError::ResourceLimit)?;
        Ok(())
    }
    pub fn begin_pass<'a>(
        &'a mut self,
        pass_index: u16,
        input: LayoutPassInput<'_>,
    ) -> Result<PassMaterializationPermit<'a>, PaginationError> {
        if pass_index != self.next_pass_index
            || input.state_index().get() != pass_index
            || !input.session.same_as(&self.session)
            || !input.layout_epoch().same_stable_inputs(self.layout_epoch)
            || input.layout_epoch().references() != input.generated_text().reference_fingerprint()
            || (pass_index == 0
                && (input.fingerprint() != self.initial_fingerprint
                    || input.layout_epoch() != self.layout_epoch))
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        self.consume_layout_pass()?;
        self.next_pass_index = self
            .next_pass_index
            .checked_add(1)
            .ok_or(PaginationError::PassIndexOverflow)?;
        let remaining_pages = self.max_pages;
        let remaining_fragments = self.max_fragments;
        let input_fingerprint = input.fingerprint();
        let layout_epoch = input.layout_epoch();
        let generated_text = input.generated_text().clone();
        Ok(PassMaterializationPermit {
            budget: self,
            session: input.session,
            pass_index,
            input_fingerprint,
            layout_epoch,
            generated_text,
            remaining_pages,
            remaining_fragments,
            pages: Vec::new(),
            active_page: None,
            next_fragment_ordinal: BTreeMap::new(),
            expected_page_start: None,
            pagination_exhausted: false,
            fallback_score: FallbackScoreAccumulator::default(),
        })
    }
    pub fn finish(self) -> PaginationBudgetReceipt {
        PaginationBudgetReceipt {
            session: self.session,
            initial_fingerprint: self.initial_fingerprint,
            layout_epoch: self.layout_epoch,
            passes: self.completed_passes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PageBudgetSummary {
    page_index: u32,
    page_start: FlowPosition,
    flow_owner: NodeId,
    content_owner: NodeId,
    master_id: MasterId,
    frames: Vec<PageFramePlan>,
    consumed_fragment_count: u64,
    fragments: Vec<PlacedFragment>,
    footnote_ids: BTreeSet<FootnoteId>,
    float_decisions: Vec<FloatDecision>,
    column_decisions: Vec<ColumnDecision>,
    resolved_references: Vec<ResolvedReference>,
    placed_anchors: Vec<PlacedAnchor>,
    continuation: Option<Continuation>,
    fragmenter_invoked: bool,
    next_fragment_cursor: FlowPosition,
    fragmentation_exhausted: bool,
    lookback_search_issued: bool,
    lookback_search_completed: bool,
}
impl PageBudgetSummary {
    fn matches(&self, page: &PagePlan) -> bool {
        self.page_index == page.page_index
            && self.master_id == page.master_id
            && self.frames == page.frames
            && u64::try_from(page.fragments.len()).ok() == Some(self.consumed_fragment_count)
            && self.fragments == page.fragments
            && self.footnote_ids.iter().cloned().collect::<Vec<_>>() == page.footnote_ids
            && self.float_decisions == page.float_decisions
            && self.column_decisions == page.column_decisions
            && self.resolved_references == page.resolved_references
    }
}

#[derive(Default)]
struct FallbackScoreAccumulator {
    hard_violations: u32,
    keep: i64,
    widow_orphan: i64,
    heading_isolation: i64,
    table_split: i64,
    footnote_split: i64,
    unused_space: i64,
    overflow: i64,
}
#[derive(Clone, Copy)]
enum CostComponentKind {
    Keep,
    WidowOrphan,
    HeadingIsolation,
    TableSplit,
    FootnoteSplit,
    UnusedSpace,
    Overflow,
}
impl FallbackScoreAccumulator {
    fn record_hard_violation(&mut self) -> Result<(), PaginationError> {
        self.hard_violations = self
            .hard_violations
            .checked_add(1)
            .ok_or(PaginationError::ArithmeticOverflow)?;
        Ok(())
    }
    fn add(&mut self, kind: CostComponentKind, value: i64) -> Result<(), PaginationError> {
        let mut values = [
            self.keep,
            self.widow_orphan,
            self.heading_isolation,
            self.table_split,
            self.footnote_split,
            self.unused_space,
            self.overflow,
        ];
        let index = kind as usize;
        let updated = values[index]
            .checked_add(value)
            .ok_or(PaginationError::ArithmeticOverflow)?;
        if !(-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&updated) {
            return Err(PaginationError::FingerprintIntegerOutOfRange);
        }
        values[index] = updated;
        let total = values
            .iter()
            .try_fold(0i64, |total, component| total.checked_add(*component));
        if !matches!(total, Some(total) if (-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&total))
        {
            return Err(PaginationError::FingerprintIntegerOutOfRange);
        }
        match kind {
            CostComponentKind::Keep => self.keep = updated,
            CostComponentKind::WidowOrphan => self.widow_orphan = updated,
            CostComponentKind::HeadingIsolation => self.heading_isolation = updated,
            CostComponentKind::TableSplit => self.table_split = updated,
            CostComponentKind::FootnoteSplit => self.footnote_split = updated,
            CostComponentKind::UnusedSpace => self.unused_space = updated,
            CostComponentKind::Overflow => self.overflow = updated,
        }
        Ok(())
    }
    fn finish(self) -> Result<FallbackScore, PaginationError> {
        let components = CostComponents::new(
            self.keep,
            self.widow_orphan,
            self.heading_isolation,
            self.table_split,
            self.footnote_split,
            self.unused_space,
            self.overflow,
        )
        .ok_or(PaginationError::FingerprintIntegerOutOfRange)?;
        Ok(FallbackScore::new(self.hard_violations, components))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct PassBudgetSummary {
    session: PaginationSessionId,
    pass_index: u16,
    input_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    generated_text: GeneratedTextStore,
    pages: Vec<PageBudgetSummary>,
    fallback_score: FallbackScore,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PassMaterializationReceipt {
    summary: PassBudgetSummary,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationBudgetReceipt {
    session: PaginationSessionId,
    initial_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    passes: Vec<PassBudgetSummary>,
}

pub struct PassMaterializationPermit<'a> {
    budget: &'a mut PaginationWorkBudget,
    session: PaginationSessionId,
    pass_index: u16,
    input_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    generated_text: GeneratedTextStore,
    remaining_pages: u32,
    remaining_fragments: u64,
    pages: Vec<PageBudgetSummary>,
    active_page: Option<PageBudgetSummary>,
    next_fragment_ordinal: BTreeMap<NodeId, u32>,
    expected_page_start: Option<FlowPosition>,
    pagination_exhausted: bool,
    fallback_score: FallbackScoreAccumulator,
}

/// Opaque proof that the designated `Fragmenter` produced the exact placed
/// fragments recorded by the active pagination pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentMaterializationReceipt {
    placed_fragments: Vec<PlacedFragment>,
    continuation: Continuation,
    discovered_footnotes: Vec<FootnoteId>,
    placed_anchors: Vec<PlacedAnchor>,
}
impl FragmentMaterializationReceipt {
    pub fn placed_fragments(&self) -> &[PlacedFragment] {
        &self.placed_fragments
    }
    pub const fn continuation(&self) -> &Continuation {
        &self.continuation
    }
    pub fn discovered_footnotes(&self) -> &[FootnoteId] {
        &self.discovered_footnotes
    }
    pub fn placed_anchors(&self) -> &[PlacedAnchor] {
        &self.placed_anchors
    }
}

impl PassMaterializationPermit<'_> {
    /// Records one hard pagination violation at the decision point that
    /// produced it. The final score has no caller-supplied aggregate input.
    pub fn record_hard_violation(&mut self) -> Result<(), PaginationError> {
        self.fallback_score.record_hard_violation()
    }
    pub fn add_keep_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score.add(CostComponentKind::Keep, value)
    }
    pub fn add_widow_orphan_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::WidowOrphan, value)
    }
    pub fn add_heading_isolation_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::HeadingIsolation, value)
    }
    pub fn add_table_split_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::TableSplit, value)
    }
    pub fn add_footnote_split_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::FootnoteSplit, value)
    }
    pub fn add_unused_space_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::UnusedSpace, value)
    }
    pub fn add_overflow_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score.add(CostComponentKind::Overflow, value)
    }
    pub fn begin_page(
        &mut self,
        page: &PageContext,
        cursor: &FlowCursor,
        frames: &[PageFramePlan],
    ) -> Result<(), PaginationError> {
        let page_index = page.page_index();
        if self.remaining_pages == 0 {
            return Err(PaginationError::ResourceLimit);
        }
        if self.active_page.is_some()
            || self.pagination_exhausted
            || self
                .expected_page_start
                .as_ref()
                .is_some_and(|expected| expected != cursor.position())
            || page.package_document_fingerprint() != self.layout_epoch.document()
            || page.package_style_fingerprint() != self.layout_epoch.style()
            || cursor.epoch() != self.layout_epoch
            || page.page_start() != cursor.position()
            || page.flow_owner() != cursor.owner_node()
            || page_index
                != u32::try_from(self.pages.len())
                    .map_err(|_| PaginationError::ArithmeticOverflow)?
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        validate_page_frames_for_context(page, frames)?;
        self.remaining_pages = self
            .remaining_pages
            .checked_sub(1)
            .ok_or(PaginationError::ResourceLimit)?;
        self.active_page = Some(PageBudgetSummary {
            page_index,
            page_start: page.page_start().clone(),
            flow_owner: page.flow_owner(),
            content_owner: page.content_owner(),
            master_id: page.master_id().clone(),
            frames: frames.to_vec(),
            consumed_fragment_count: 0,
            fragments: Vec::new(),
            footnote_ids: BTreeSet::new(),
            float_decisions: Vec::new(),
            column_decisions: Vec::new(),
            resolved_references: Vec::new(),
            placed_anchors: Vec::new(),
            continuation: None,
            fragmenter_invoked: false,
            next_fragment_cursor: page.page_start().clone(),
            fragmentation_exhausted: false,
            lookback_search_issued: false,
            lookback_search_completed: false,
        });
        Ok(())
    }
    pub fn finish_page(&mut self, page: &PagePlan) -> Result<(), PaginationError> {
        if self
            .budget
            .dequeued_floats
            .iter()
            .any(|(pass_index, _, _)| *pass_index == self.pass_index)
            || self
                .budget
                .column_candidates
                .keys()
                .any(|(pass_index, _)| *pass_index == self.pass_index)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let active = self
            .active_page
            .take()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        let page_reflows = self
            .budget
            .footnote_reflows
            .get(&(self.pass_index, active.page_index))
            .copied()
            .unwrap_or(0);
        if !active.matches(page)
            || active.lookback_search_issued != active.lookback_search_completed
            || (page_reflows > 0 && active.footnote_ids.is_empty())
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        match active.continuation.as_ref() {
            Some(Continuation::More(cursor)) => {
                self.expected_page_start = Some(cursor.position().clone());
            }
            Some(Continuation::Exhausted(_)) | None => {
                self.expected_page_start = None;
                self.pagination_exhausted = true;
            }
        }
        self.pages.push(active);
        Ok(())
    }
    /// Runs the sole fragment owner and records its exact materialization
    /// before the caller can build a `PagePlan`. `finish_page` accepts only the
    /// recorded fragment sequence.
    pub fn run_fragmenter<F: Fragmenter>(
        &mut self,
        fragmenter: &F,
        request: &FragmentRequest<'_>,
        frame_kind: PageFrameKind,
        column_index: u32,
    ) -> Result<FragmentMaterializationReceipt, FragmentError> {
        request.validate()?;
        // The reference implementation currently owns only the main/body
        // flow. Header/footer are reserved geometry and footnotes require an
        // independent subflow cursor/terminal receipt; none may consume the
        // body continuation accidentally.
        if frame_kind != PageFrameKind::Body {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let before = self
            .active_page
            .as_ref()
            .ok_or(FragmentError::ResourceLimit)?
            .consumed_fragment_count;
        let active = self
            .active_page
            .as_ref()
            .ok_or(FragmentError::ResourceLimit)?;
        if active.fragmentation_exhausted
            || request.cursor().position() != &active.next_fragment_cursor
            || request.page().page_index() != active.page_index
            || request.page().master_id() != &active.master_id
            || request.page().page_start() != &active.page_start
            || !active.frames.iter().any(|frame| {
                frame.kind == frame_kind
                    && frame.column_index == column_index
                    && frame.bounds == request.frame()
            })
        {
            return Err(FragmentError::InvalidPageContext);
        }
        let result = fragmenter.fragment(request, self)?;
        result.validate_progress(request)?;
        if !result.discovered_footnotes.is_empty() {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let active = self
            .active_page
            .as_mut()
            .ok_or(FragmentError::ResourceLimit)?;
        let consumed = active
            .consumed_fragment_count
            .checked_sub(before)
            .ok_or(FragmentError::ArithmeticOverflow)?;
        if consumed
            != u64::try_from(result.fragments.len())
                .map_err(|_| FragmentError::ArithmeticOverflow)?
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let mut placed_fragments = Vec::with_capacity(result.fragments.len());
        for fragment in &result.fragments {
            let owner = fragment.start().owner();
            let ordinal = self.next_fragment_ordinal.entry(owner).or_insert(0);
            let placed = PlacedFragment {
                start: fragment.start().clone(),
                end: fragment.end().clone(),
                owner,
                owner_local_ordinal: *ordinal,
                frame_kind,
                column_index,
                bounds: fragment.bounds(),
            };
            *ordinal = ordinal
                .checked_add(1)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            active.fragments.push(placed.clone());
            placed_fragments.push(placed);
        }
        let mut discovered_footnotes = result.discovered_footnotes;
        discovered_footnotes.sort();
        if discovered_footnotes.windows(2).any(|ids| ids[0] == ids[1]) {
            return Err(FragmentError::InvalidFragmentKey);
        }
        active
            .footnote_ids
            .extend(discovered_footnotes.iter().cloned());
        let mut discovered_anchors = result.discovered_anchors;
        discovered_anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        if discovered_anchors
            .windows(2)
            .any(|anchors| anchors[0].anchor_id == anchors[1].anchor_id)
            || active.placed_anchors.iter().any(|existing| {
                discovered_anchors
                    .iter()
                    .any(|anchor| &anchor.anchor_id == existing.anchor_id())
            })
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let placed_anchors = discovered_anchors
            .iter()
            .map(|anchor| {
                PlacedAnchor::new(
                    anchor,
                    active.page_index,
                    frame_kind,
                    column_index,
                    request.frame(),
                )
            })
            .collect::<Result<Vec<_>, FragmentError>>()?;
        active.placed_anchors.extend(placed_anchors.clone());
        active
            .placed_anchors
            .sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        active.continuation = Some(result.continuation.clone());
        match &result.continuation {
            Continuation::More(cursor) => {
                active.next_fragment_cursor = cursor.position().clone();
            }
            Continuation::Exhausted(cursor) => {
                active.next_fragment_cursor = cursor.position().clone();
                active.fragmentation_exhausted = true;
            }
        }
        active.fragmenter_invoked = true;
        Ok(FragmentMaterializationReceipt {
            placed_fragments,
            continuation: result.continuation,
            discovered_footnotes,
            placed_anchors,
        })
    }
    /// Finalizes a float that was enqueued and then dequeued by fragment work.
    /// The exact decision becomes part of the page materialization receipt.
    pub fn record_float_decision(
        &mut self,
        decision: FloatDecision,
    ) -> Result<(), PaginationError> {
        let key = (
            self.pass_index,
            decision.owner,
            decision.owner_local_ordinal,
        );
        let active = self
            .active_page
            .as_ref()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if !self.budget.dequeued_floats.contains(&key)
            || active.float_decisions.iter().any(|existing| {
                (existing.owner, existing.owner_local_ordinal)
                    == (decision.owner, decision.owner_local_ordinal)
            })
            || !active.frames.iter().any(|frame| {
                frame.kind == decision.frame_kind
                    && frame.column_index == decision.column_index
                    && rect_contains(frame.bounds, decision.bounds)
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        validate_rect(decision.bounds)?;
        self.budget.dequeued_floats.remove(&key);
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        active.float_decisions.push(decision);
        active
            .float_decisions
            .sort_by_key(|value| (value.owner, value.owner_local_ordinal));
        Ok(())
    }
    /// Finalizes the canonical column choice for one container after at least
    /// one budgeted balance candidate was evaluated.
    pub fn record_column_decisions(
        &mut self,
        container: NodeId,
        mut decisions: Vec<ColumnDecision>,
    ) -> Result<(), PaginationError> {
        let active = self
            .active_page
            .as_ref()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if self
            .budget
            .column_candidates
            .get(&(self.pass_index, container))
            .copied()
            .unwrap_or(0)
            == 0
            || decisions.is_empty()
            || active
                .column_decisions
                .iter()
                .any(|existing| existing.container == container)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        decisions.sort_by_key(|decision| decision.column_index);
        for (expected, decision) in decisions.iter().enumerate() {
            if decision.container != container
                || decision.column_index
                    != u32::try_from(expected).map_err(|_| PaginationError::ArithmeticOverflow)?
                || !active.frames.iter().any(|frame| {
                    frame.kind == PageFrameKind::Body
                        && rect_contains(frame.bounds, decision.bounds)
                })
            {
                return Err(PaginationError::InvalidWorkPermit);
            }
            validate_rect(decision.bounds)?;
        }
        self.budget
            .column_candidates
            .remove(&(self.pass_index, container));
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        active.column_decisions.extend(decisions);
        active
            .column_decisions
            .sort_by_key(|decision| (decision.container, decision.column_index));
        Ok(())
    }
    /// Records one store-issued reference result in canonical order.
    pub fn record_resolved_reference(
        &mut self,
        reference: ResolvedReference,
    ) -> Result<(), PaginationError> {
        if reference.reference_fingerprint != self.layout_epoch.references() {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if active
            .resolved_references
            .iter()
            .any(|existing| existing.provenance == reference.provenance)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        active.resolved_references.push(reference);
        active
            .resolved_references
            .sort_by(compare_resolved_reference);
        Ok(())
    }
    pub fn begin_page_break_search(&mut self) -> Result<PageBreakSearchBudget, PaginationError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if active.lookback_search_issued {
            return Err(PaginationError::InvalidWorkPermit);
        }
        active.lookback_search_issued = true;
        Ok(PageBreakSearchBudget {
            page_index: active.page_index,
            boundary: active.page_start.clone(),
            remaining_candidates: self.budget.max_page_break_lookback,
        })
    }
    pub fn finish_page_break_search(
        &mut self,
        search: PageBreakSearchBudget,
    ) -> Result<(), PaginationError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if !active.lookback_search_issued
            || active.lookback_search_completed
            || active.page_index != search.page_index
            || active.page_start != search.boundary
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        active.lookback_search_completed = true;
        Ok(())
    }
    pub fn finish(
        self,
        flow: &FlowTree,
        pages: &[PagePlan],
    ) -> Result<PassMaterializationReceipt, PaginationError> {
        if self.active_page.is_some()
            || flow.epoch() != self.layout_epoch
            || self
                .budget
                .queued_floats
                .iter()
                .any(|(pass_index, _, _)| *pass_index == self.pass_index)
            || self
                .budget
                .dequeued_floats
                .iter()
                .any(|(pass_index, _, _)| *pass_index == self.pass_index)
            || self
                .budget
                .column_candidates
                .keys()
                .any(|(pass_index, _)| *pass_index == self.pass_index)
            || pages.len() != self.pages.len()
            || self
                .pages
                .iter()
                .any(|summary| summary.page_start.epoch() != self.layout_epoch)
            || pages
                .iter()
                .zip(&self.pages)
                .any(|(page, summary)| !summary.matches(page))
            || !validate_materialization_chain(flow, &self.pages)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let summary = PassBudgetSummary {
            session: self.session,
            pass_index: self.pass_index,
            input_fingerprint: self.input_fingerprint,
            layout_epoch: self.layout_epoch,
            generated_text: self.generated_text,
            pages: self.pages,
            fallback_score: self.fallback_score.finish()?,
        };
        self.budget.completed_passes.push(summary.clone());
        Ok(PassMaterializationReceipt { summary })
    }
}
impl FragmentWorkBudget for PassMaterializationPermit<'_> {
    fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(FragmentError::ResourceLimit)?;
        self.remaining_fragments = self
            .remaining_fragments
            .checked_sub(count)
            .ok_or(FragmentError::ResourceLimit)?;
        active.consumed_fragment_count = active
            .consumed_fragment_count
            .checked_add(count)
            .ok_or(FragmentError::ArithmeticOverflow)?;
        Ok(())
    }
    fn consume_footnote_reflow(&mut self, page_index: u32) -> Result<(), FragmentError> {
        if self.active_page.as_ref().map(|page| page.page_index) != Some(page_index) {
            return Err(FragmentError::InvalidPageContext);
        }
        let used = self
            .budget
            .footnote_reflows
            .entry((self.pass_index, page_index))
            .or_insert(0);
        *used = used
            .checked_add(1)
            .filter(|used| *used <= self.budget.max_footnote_reflows_per_page)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }
    fn consume_column_candidate(&mut self, container: NodeId) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        let used = self
            .budget
            .column_candidates
            .entry((self.pass_index, container))
            .or_insert(0);
        *used = used
            .checked_add(1)
            .filter(|used| *used <= self.budget.max_column_balance_candidates)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }
    fn enqueue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        let key = (self.pass_index, owner, owner_local_ordinal);
        if self.budget.queued_floats.contains(&key) {
            return Err(FragmentError::InvalidFloatState);
        }
        if self.budget.queued_floats.len() >= self.budget.max_float_queue as usize {
            return Err(FragmentError::ResourceLimit);
        }
        self.budget.queued_floats.insert(key);
        Ok(())
    }
    fn dequeue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        let key = (self.pass_index, owner, owner_local_ordinal);
        if !self.budget.queued_floats.remove(&key) || !self.budget.dequeued_floats.insert(key) {
            return Err(FragmentError::InvalidFloatState);
        }
        Ok(())
    }
    fn consume_float_carry(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        if !self
            .budget
            .queued_floats
            .contains(&(self.pass_index, owner, owner_local_ordinal))
        {
            return Err(FragmentError::InvalidFloatState);
        }
        let used = self
            .budget
            .float_carries
            .entry((self.pass_index, owner, owner_local_ordinal))
            .or_insert(0);
        *used = used
            .checked_add(1)
            .filter(|used| *used <= self.budget.max_float_carry_pages)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PageBreakSearchBudget {
    page_index: u32,
    boundary: FlowPosition,
    remaining_candidates: u16,
}
impl PageBreakSearchBudget {
    pub fn consume_candidate(&mut self) -> Result<(), PaginationError> {
        self.remaining_candidates = self
            .remaining_candidates
            .checked_sub(1)
            .ok_or(PaginationError::ResourceLimit)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialPaginationState {
    layout_epoch: LayoutEpoch,
    flow_positions: Vec<FlowPosition>,
    generated_text: GeneratedTextStore,
    fingerprint: LayoutStateFingerprint,
}
impl InitialPaginationState {
    pub fn new(
        flow: &FlowTree,
        package: &ValidatedParsedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, PaginationError> {
        if package.document_nodes().generated_sites().len() != 0 {
            return Err(PaginationError::UnsupportedReferenceTransition);
        }
        let generated_text = GeneratedTextStore::new(
            Vec::new(),
            package.document_nodes(),
            limits,
            &package.package().text_store,
        )
        .map_err(|_| PaginationError::InvalidInitialReferenceSeed)?;
        let layout_epoch = flow.epoch();
        if layout_epoch.document() != package.epoch_identity().document()
            || layout_epoch.style() != package.epoch_identity().style()
            || layout_epoch.references() != generated_text.reference_fingerprint()
        {
            return Err(PaginationError::EpochMismatch);
        }
        let mut jcs = String::from("{\"algorithm\":");
        push_jcs_string(&mut jcs, LayoutStateFingerprint::INITIAL_ALGORITHM_ID);
        jcs.push_str(",\"flow_positions\":[");
        encode_flow_positions(&mut jcs, flow.positions());
        jcs.push_str("],\"layout_epoch\":");
        encode_layout_epoch(&mut jcs, layout_epoch);
        jcs.push_str(",\"resolved_generated_text\":[");
        for (index, buffer) in generated_text.buffers().iter().enumerate() {
            comma(&mut jcs, index);
            jcs.push_str("{\"end_byte\":");
            jcs.push_str(&buffer.utf8().len().to_string());
            jcs.push_str(",\"key\":");
            encode_generated_key(&mut jcs, buffer.key());
            jcs.push_str(",\"start_byte\":0,\"utf8\":");
            push_jcs_string(&mut jcs, buffer.utf8());
            jcs.push('}');
        }
        jcs.push_str("]}");
        let fingerprint = initial_pagination_state_fingerprint_from_jcs(&jcs);
        Ok(Self {
            layout_epoch,
            flow_positions: flow.positions().to_vec(),
            generated_text,
            fingerprint,
        })
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub const fn generated_text(&self) -> &GeneratedTextStore {
        &self.generated_text
    }
    pub fn flow_positions(&self) -> &[FlowPosition] {
        &self.flow_positions
    }
    pub const fn fingerprint(&self) -> LayoutStateFingerprint {
        self.fingerprint
    }
}

/// Sealed proof that the next pass overlay was derived from one exact
/// materialized predecessor. The reference implementation can issue this only
/// for documents with no generated sites; nonempty feedback remains
/// fail-closed until the reference resolver owns the page-to-overlay
/// transition.
#[derive(Debug, Eq, PartialEq)]
pub struct ReferenceTransitionReceipt<'a> {
    session: PaginationSessionId,
    previous_state: MaterializedStateIndex,
    previous_fingerprint: LayoutStateFingerprint,
    working_epoch: LayoutEpoch,
    generated_text: &'a GeneratedTextStore,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PaginationInput<'a> {
    session: PaginationSessionId,
    initial_state: InitialPaginationState,
    package_context: &'a PackagePaginationContext,
    options: PaginationOptions,
    work_budget: Option<PaginationWorkBudget>,
}
impl<'a> PaginationInput<'a> {
    pub fn new(
        initial_state: InitialPaginationState,
        package_context: &'a PackagePaginationContext,
        options: PaginationOptions,
    ) -> Result<Self, PaginationError> {
        if options.max_pages == 0
            || options.max_layout_passes == 0
            || options.max_page_break_lookback == 0
        {
            return Err(PaginationError::InvalidOptions);
        }
        if initial_state.layout_epoch().document() != package_context.document_fingerprint()
            || initial_state.layout_epoch().style() != package_context.style_fingerprint()
        {
            return Err(PaginationError::PackageEpochMismatch);
        }
        package_context
            .page_masters()
            .validate()
            .map_err(|_| PaginationError::MissingDefaultMaster)?;
        let session = PaginationSessionId::issue();
        let work_budget = PaginationWorkBudget::new(
            session.clone(),
            options,
            initial_state.fingerprint(),
            initial_state.layout_epoch(),
        );
        Ok(Self {
            session,
            initial_state,
            package_context,
            options,
            work_budget: Some(work_budget),
        })
    }
    pub const fn initial_fingerprint(&self) -> LayoutStateFingerprint {
        self.initial_state.fingerprint()
    }
    pub const fn initial_state(&self) -> &InitialPaginationState {
        &self.initial_state
    }
    pub const fn page_masters(&self) -> &PageMasterSet {
        self.package_context.page_masters()
    }
    pub const fn options(&self) -> PaginationOptions {
        self.options
    }
    pub fn take_work_budget(&mut self) -> Result<PaginationWorkBudget, PaginationError> {
        self.work_budget
            .take()
            .ok_or(PaginationError::WorkBudgetAlreadyIssued)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPassInput<'a> {
    session: PaginationSessionId,
    state_index: LayoutStateIndex,
    fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    generated_text: &'a GeneratedTextStore,
}
impl<'a> LayoutPassInput<'a> {
    pub fn initial(input: &'a PaginationInput<'_>) -> Self {
        Self {
            session: input.session.clone(),
            state_index: LayoutStateIndex::INITIAL,
            fingerprint: input.initial_fingerprint(),
            layout_epoch: input.initial_state().layout_epoch(),
            generated_text: input.initial_state().generated_text(),
        }
    }
    pub fn transitioned(transition: ReferenceTransitionReceipt<'a>) -> Self {
        Self {
            session: transition.session,
            state_index: LayoutStateIndex::new(transition.previous_state.get()),
            fingerprint: transition.previous_fingerprint,
            layout_epoch: transition.working_epoch,
            generated_text: transition.generated_text,
        }
    }
    pub const fn state_index(&self) -> LayoutStateIndex {
        self.state_index
    }
    pub const fn fingerprint(&self) -> LayoutStateFingerprint {
        self.fingerprint
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub const fn generated_text(&self) -> &'a GeneratedTextStore {
        self.generated_text
    }
}

/// State-dependent layout rebuilt for one pagination pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedLayout {
    flow: FlowTree,
    initial_cursor: FlowCursor,
}
impl PreparedLayout {
    pub fn new(
        input: LayoutPassInput<'_>,
        flow: FlowTree,
        initial_cursor: FlowCursor,
    ) -> Result<Self, PaginationError> {
        if flow.epoch() != initial_cursor.epoch() || flow.epoch() != input.layout_epoch() {
            return Err(PaginationError::InvalidPreparedLayout);
        }
        Ok(Self {
            flow,
            initial_cursor,
        })
    }
    pub const fn flow(&self) -> &FlowTree {
        &self.flow
    }
    pub const fn initial_cursor(&self) -> &FlowCursor {
        &self.initial_cursor
    }
}

pub trait LayoutPassProvider {
    fn prepare(
        &self,
        input: LayoutPassInput<'_>,
        permit: &mut PassMaterializationPermit<'_>,
    ) -> Result<PreparedLayout, PaginationError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacedFragment {
    pub start: FlowPosition,
    pub end: FlowPosition,
    pub owner: NodeId,
    pub owner_local_ordinal: u32,
    pub frame_kind: PageFrameKind,
    pub column_index: u32,
    pub bounds: Rect,
}

/// Anchor placement bound to the exact page/frame selected by the pagination
/// receipt. Display destinations are derived from this record, not from a
/// caller-provided anchor list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacedAnchor {
    anchor_id: AnchorId,
    owner_node: NodeId,
    page_index: u32,
    frame_kind: PageFrameKind,
    column_index: u32,
    position_in_frame: Point,
}
impl PlacedAnchor {
    fn new(
        anchor: &DiscoveredAnchor,
        page_index: u32,
        frame_kind: PageFrameKind,
        column_index: u32,
        frame: Rect,
    ) -> Result<Self, FragmentError> {
        let x = anchor.position_in_frame.x.raw();
        let y = anchor.position_in_frame.y.raw();
        if x < 0 || y < 0 || x > frame.width().get().raw() || y > frame.height().get().raw() {
            return Err(FragmentError::InvalidPageContext);
        }
        Ok(Self {
            anchor_id: anchor.anchor_id.clone(),
            owner_node: anchor.owner_node,
            page_index,
            frame_kind,
            column_index,
            position_in_frame: anchor.position_in_frame,
        })
    }
    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }
    pub const fn owner_node(&self) -> NodeId {
        self.owner_node
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_kind(&self) -> PageFrameKind {
        self.frame_kind
    }
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn position_in_frame(&self) -> Point {
        self.position_in_frame
    }
    pub fn position_on_page(&self, frame: Rect) -> Option<Point> {
        Some(Point {
            x: frame.x().checked_add(self.position_in_frame.x)?,
            y: frame.y().checked_add(self.position_in_frame.y)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PageFrameKind {
    Body,
    Header,
    Footer,
    Footnote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageFramePlan {
    pub kind: PageFrameKind,
    pub column_index: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FloatDecision {
    pub owner: NodeId,
    pub owner_local_ordinal: u32,
    pub frame_kind: PageFrameKind,
    pub column_index: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnDecision {
    pub container: NodeId,
    pub column_index: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PagePlan {
    pub page_index: u32,
    pub master_id: MasterId,
    pub frames: Vec<PageFramePlan>,
    pub fragments: Vec<PlacedFragment>,
    pub footnote_ids: Vec<FootnoteId>,
    pub float_decisions: Vec<FloatDecision>,
    pub column_decisions: Vec<ColumnDecision>,
    pub resolved_references: Vec<ResolvedReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedReference {
    anchor_id: AnchorId,
    provenance: GeneratedProvenance,
    reference_fingerprint: ReferenceFingerprint,
    utf8: String,
}
impl ResolvedReference {
    pub fn from_store(
        store: &GeneratedTextStore,
        key: GeneratedBufferKey,
        start: Utf8ByteOffset,
        end: Utf8ByteOffset,
    ) -> Result<Self, ResolvedReferenceError> {
        let reference_site = store
            .document_nodes()
            .generated_site(key)
            .ok_or(ResolvedReferenceError::InvalidReferenceSite)?;
        let anchor_id = match reference_site.target() {
            GeneratedSiteTarget::Anchor(anchor_id)
                if key.generation_kind() == GenerationKind::PageReference =>
            {
                anchor_id.clone()
            }
            _ => return Err(ResolvedReferenceError::InvalidReferenceSite),
        };
        let provenance = store.provenance(key, start, end)?;
        let buffer = store
            .get(provenance.text_span().text_id())
            .ok_or(GeneratedTextStoreError::UnknownKey)?;
        let utf8 = buffer.utf8()[start.get() as usize..end.get() as usize].to_owned();
        Ok(Self {
            anchor_id,
            provenance,
            reference_fingerprint: store.reference_fingerprint(),
            utf8,
        })
    }
    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }
    pub const fn provenance(&self) -> GeneratedProvenance {
        self.provenance
    }
    pub fn utf8(&self) -> &str {
        &self.utf8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedReferenceError {
    InvalidReferenceSite,
    GeneratedText(GeneratedTextStoreError),
}
impl From<GeneratedTextStoreError> for ResolvedReferenceError {
    fn from(value: GeneratedTextStoreError) -> Self {
        Self::GeneratedText(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationFingerprintRecord {
    layout_epoch: LayoutEpoch,
    placed_anchors: Vec<PlacedAnchor>,
    flow_positions: Vec<FlowPosition>,
    pages: Vec<PagePlan>,
    generated_text: GeneratedTextStore,
}
impl PaginationFingerprintRecord {
    pub fn new(
        flow: &FlowTree,
        mut pages: Vec<PagePlan>,
        generated_store: GeneratedTextStore,
        mut placed_anchors: Vec<PlacedAnchor>,
    ) -> Result<Self, PaginationError> {
        let layout_epoch = flow.epoch();
        if layout_epoch.references() != generated_store.reference_fingerprint() {
            return Err(PaginationError::EpochMismatch);
        }
        if generated_store
            .buffers()
            .iter()
            .any(|buffer| !flow.contains_owner(buffer.key().owner()))
        {
            return Err(PaginationError::UnknownFlowOwner);
        }
        if pages.is_empty() {
            return Err(PaginationError::EmptyPages);
        }
        canonicalize_placed_anchors(&mut placed_anchors);
        if !validate_fingerprint_anchors(flow, &pages, &placed_anchors) {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let mut all_provenance = BTreeSet::new();
        let mut materialized_footnotes = BTreeSet::new();
        let mut next_fragment_ordinal = BTreeMap::<NodeId, u32>::new();
        for (expected, page) in pages.iter_mut().enumerate() {
            if page.page_index != u32::try_from(expected).map_err(|_| PaginationError::PageLimit)? {
                return Err(PaginationError::InvalidPageIndex);
            }
            page.frames
                .sort_by_key(|frame| (frame.kind, frame.column_index));
            if !matches!(
                page.frames.first(),
                Some(frame) if frame.kind == PageFrameKind::Body && frame.column_index == 0
            ) {
                return Err(PaginationError::MissingBodyFrame);
            }
            let mut previous_frame: Option<(PageFrameKind, u32)> = None;
            let mut frame_bounds = BTreeMap::new();
            for frame in &page.frames {
                validate_rect(frame.bounds)?;
                let expected_column = match previous_frame {
                    Some((kind, column)) if kind == frame.kind => column
                        .checked_add(1)
                        .ok_or(PaginationError::InvalidFrameColumn)?,
                    _ => 0,
                };
                if frame.column_index != expected_column {
                    return Err(PaginationError::InvalidFrameColumn);
                }
                previous_frame = Some((frame.kind, frame.column_index));
                frame_bounds.insert((frame.kind, frame.column_index), frame.bounds);
            }
            page.fragments.sort_by(|left, right| {
                compare_flow_position(&left.start, &right.start)
                    .then_with(|| left.owner.cmp(&right.owner))
                    .then_with(|| left.owner_local_ordinal.cmp(&right.owner_local_ordinal))
            });
            if page.fragments.windows(2).any(|pair| {
                compare_flow_position(&pair[0].start, &pair[1].start).is_eq()
                    && pair[0].owner == pair[1].owner
                    && pair[0].owner_local_ordinal == pair[1].owner_local_ordinal
            }) {
                return Err(PaginationError::DuplicateFragmentPosition);
            }
            for fragment in &page.fragments {
                let expected_ordinal = next_fragment_ordinal.entry(fragment.owner).or_insert(0);
                if fragment.start.epoch() != layout_epoch
                    || fragment.end.epoch() != layout_epoch
                    || !flow.contains_position(&fragment.start)
                    || !flow.contains_position(&fragment.end)
                    || !fragment
                        .start
                        .cmp_within_epoch(&fragment.end)
                        .map_err(|_| PaginationError::EpochMismatch)?
                        .is_lt()
                    || fragment.start.owner() != fragment.owner
                    || fragment.owner_local_ordinal != *expected_ordinal
                {
                    return Err(PaginationError::InvalidFragmentRange);
                }
                *expected_ordinal = expected_ordinal
                    .checked_add(1)
                    .ok_or(PaginationError::ArithmeticOverflow)?;
                validate_flow_position(&fragment.start)?;
                validate_flow_position(&fragment.end)?;
                validate_rect(fragment.bounds)?;
                let containing_frame = frame_bounds
                    .get(&(fragment.frame_kind, fragment.column_index))
                    .ok_or(PaginationError::UnknownFrameReference)?;
                if !rect_contains(*containing_frame, fragment.bounds) {
                    return Err(PaginationError::FragmentOutsideFrame);
                }
            }
            page.footnote_ids.sort();
            if page.footnote_ids.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(PaginationError::DuplicateFootnote);
            }
            materialized_footnotes.extend(page.footnote_ids.iter().cloned());
            page.float_decisions
                .sort_by_key(|decision| (decision.owner, decision.owner_local_ordinal));
            if page.float_decisions.windows(2).any(|pair| {
                (pair[0].owner, pair[0].owner_local_ordinal)
                    == (pair[1].owner, pair[1].owner_local_ordinal)
            }) {
                return Err(PaginationError::DuplicateFloatDecision);
            }
            for decision in &page.float_decisions {
                if !flow.contains_owner(decision.owner) {
                    return Err(PaginationError::UnknownFlowOwner);
                }
                validate_rect(decision.bounds)?;
                let containing_frame = frame_bounds
                    .get(&(decision.frame_kind, decision.column_index))
                    .ok_or(PaginationError::UnknownFrameReference)?;
                if !rect_contains(*containing_frame, decision.bounds) {
                    return Err(PaginationError::FragmentOutsideFrame);
                }
            }
            page.column_decisions
                .sort_by_key(|decision| (decision.container, decision.column_index));
            let mut previous_column: Option<(NodeId, u32)> = None;
            for decision in &page.column_decisions {
                if !flow.contains_owner(decision.container) {
                    return Err(PaginationError::UnknownFlowOwner);
                }
                let expected_column = match previous_column {
                    Some((container, column)) if container == decision.container => column
                        .checked_add(1)
                        .ok_or(PaginationError::InvalidColumnDecision)?,
                    _ => 0,
                };
                if decision.column_index != expected_column {
                    return Err(PaginationError::InvalidColumnDecision);
                }
                validate_rect(decision.bounds)?;
                previous_column = Some((decision.container, decision.column_index));
            }
            page.resolved_references.sort_by(|left, right| {
                let left_span = left.provenance.text_span().range();
                let right_span = right.provenance.text_span().range();
                (
                    left.provenance.buffer_key(),
                    left_span.start_byte(),
                    left_span.end_byte(),
                    &left.anchor_id,
                )
                    .cmp(&(
                        right.provenance.buffer_key(),
                        right_span.start_byte(),
                        right_span.end_byte(),
                        &right.anchor_id,
                    ))
            });
            for reference in &page.resolved_references {
                if reference.reference_fingerprint != generated_store.reference_fingerprint()
                    || !generated_store.validates_provenance(reference.provenance)
                    || !all_provenance.insert(reference.provenance)
                {
                    return Err(PaginationError::DuplicateResolvedReference);
                }
            }
        }
        if &materialized_footnotes
            != generated_store
                .document_nodes()
                .footnote_reference_targets()
        {
            return Err(PaginationError::InvalidFootnoteClosure);
        }
        Ok(Self {
            layout_epoch,
            placed_anchors,
            flow_positions: flow.positions().to_vec(),
            pages,
            generated_text: generated_store,
        })
    }
    pub fn pages(&self) -> &[PagePlan] {
        &self.pages
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub const fn generated_text(&self) -> &GeneratedTextStore {
        &self.generated_text
    }
    pub fn fingerprint(&self) -> LayoutStateFingerprint {
        materialized_pagination_state_fingerprint_from_jcs(&self.to_jcs())
    }

    fn to_jcs(&self) -> String {
        let mut output = String::from("{\"algorithm\":");
        push_jcs_string(
            &mut output,
            LayoutStateFingerprint::MATERIALIZED_ALGORITHM_ID,
        );
        output.push_str(",\"flow_positions\":[");
        encode_flow_positions(&mut output, &self.flow_positions);
        output.push_str("],\"layout_epoch\":");
        encode_layout_epoch(&mut output, self.layout_epoch);
        output.push_str(",\"pages\":[");
        for (page_index, page) in self.pages.iter().enumerate() {
            if page_index > 0 {
                output.push(',');
            }
            encode_page(&mut output, page);
        }
        output.push_str("],\"placed_anchors\":[");
        for (index, anchor) in self.placed_anchors.iter().enumerate() {
            comma(&mut output, index);
            encode_placed_anchor(&mut output, anchor);
        }
        output.push_str("],\"resolved_generated_text\":[");
        for (index, buffer) in self.generated_text.buffers().iter().enumerate() {
            comma(&mut output, index);
            output.push_str("{\"end_byte\":");
            output.push_str(&buffer.utf8().len().to_string());
            output.push_str(",\"key\":");
            encode_generated_key(&mut output, buffer.key());
            output.push_str(",\"start_byte\":0,\"utf8\":");
            push_jcs_string(&mut output, buffer.utf8());
            output.push('}');
        }
        output.push_str("]}");
        output
    }
}

fn compare_flow_position(left: &FlowPosition, right: &FlowPosition) -> core::cmp::Ordering {
    (
        left.global_flow_ordinal(),
        left.owner(),
        left.block_child_path(),
        left.owner_local_boundary(),
    )
        .cmp(&(
            right.global_flow_ordinal(),
            right.owner(),
            right.block_child_path(),
            right.owner_local_boundary(),
        ))
}

fn canonicalize_placed_anchors(anchors: &mut [PlacedAnchor]) {
    anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
}

fn validate_materialization_chain(flow: &FlowTree, pages: &[PageBudgetSummary]) -> bool {
    let Some(first_position) = flow.positions().first() else {
        return false;
    };
    let Some(terminal_position) = flow.positions().last() else {
        return false;
    };
    if pages.first().map(|page| &page.page_start) != Some(first_position) {
        return false;
    }

    // The canonical empty document is one default blank page. Its sole flow
    // position is both the document start and terminal boundary, so it needs
    // no synthetic Fragmenter result.
    if flow.positions().len() == 1 {
        return pages.len() == 1 && !pages[0].fragmenter_invoked && pages[0].continuation.is_none();
    }

    for (index, page) in pages.iter().enumerate() {
        if !page.fragmenter_invoked {
            return false;
        }
        match page.continuation.as_ref() {
            Some(Continuation::More(cursor)) => {
                if cursor.epoch() != flow.epoch()
                    || cursor.is_end()
                    || !flow.contains_position(cursor.position())
                    || pages.get(index + 1).map(|next| &next.page_start) != Some(cursor.position())
                {
                    return false;
                }
            }
            Some(Continuation::Exhausted(cursor)) => {
                if index + 1 != pages.len()
                    || cursor.epoch() != flow.epoch()
                    || !cursor.is_end()
                    || cursor.position() != terminal_position
                {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

fn validate_materialized_anchors(flow: &FlowTree, pages: &[PageBudgetSummary]) -> bool {
    let mut ids = BTreeSet::new();
    for page in pages {
        for anchor in &page.placed_anchors {
            let Some(frame) = page.frames.iter().find(|frame| {
                frame.kind == anchor.frame_kind && frame.column_index == anchor.column_index
            }) else {
                return false;
            };
            if anchor.page_index != page.page_index
                || flow.anchor_owner(&anchor.anchor_id) != Some(anchor.owner_node)
                || !ids.insert(anchor.anchor_id.clone())
                || anchor.position_on_page(frame.bounds).is_none()
                || anchor.position_in_frame.x.raw() < 0
                || anchor.position_in_frame.y.raw() < 0
                || anchor.position_in_frame.x.raw() > frame.bounds.width().get().raw()
                || anchor.position_in_frame.y.raw() > frame.bounds.height().get().raw()
            {
                return false;
            }
        }
    }
    ids == flow
        .anchors()
        .map(|(anchor_id, _)| anchor_id.clone())
        .collect()
}

fn validate_fingerprint_anchors(
    flow: &FlowTree,
    pages: &[PagePlan],
    anchors: &[PlacedAnchor],
) -> bool {
    let mut ids = BTreeSet::new();
    let valid = anchors.iter().all(|anchor| {
        let Some(page) = pages.get(anchor.page_index as usize) else {
            return false;
        };
        if page.page_index != anchor.page_index {
            return false;
        }
        let Some(frame) = page.frames.iter().find(|frame| {
            frame.kind == anchor.frame_kind && frame.column_index == anchor.column_index
        }) else {
            return false;
        };
        flow.anchor_owner(&anchor.anchor_id) == Some(anchor.owner_node)
            && ids.insert(anchor.anchor_id.clone())
            && anchor.position_on_page(frame.bounds).is_some()
            && anchor.position_in_frame.x.raw() >= 0
            && anchor.position_in_frame.y.raw() >= 0
            && anchor.position_in_frame.x.raw() <= frame.bounds.width().get().raw()
            && anchor.position_in_frame.y.raw() <= frame.bounds.height().get().raw()
    });
    valid
        && ids
            == flow
                .anchors()
                .map(|(anchor_id, _)| anchor_id.clone())
                .collect()
}

fn compare_resolved_reference(
    left: &ResolvedReference,
    right: &ResolvedReference,
) -> core::cmp::Ordering {
    let left_span = left.provenance.text_span().range();
    let right_span = right.provenance.text_span().range();
    (
        left.provenance.buffer_key(),
        left_span.start_byte(),
        left_span.end_byte(),
        &left.anchor_id,
    )
        .cmp(&(
            right.provenance.buffer_key(),
            right_span.start_byte(),
            right_span.end_byte(),
            &right.anchor_id,
        ))
}

fn validate_flow_position(position: &FlowPosition) -> Result<(), PaginationError> {
    if position.global_flow_ordinal() > JSON_SAFE_INTEGER_MAX as u64 {
        return Err(PaginationError::FingerprintIntegerOutOfRange);
    }
    Ok(())
}

fn validate_rect(rect: Rect) -> Result<(), PaginationError> {
    for value in [
        rect.x().raw(),
        rect.y().raw(),
        rect.width().get().raw(),
        rect.height().get().raw(),
    ] {
        if !(-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&value) {
            return Err(PaginationError::FingerprintIntegerOutOfRange);
        }
    }
    Ok(())
}

fn encode_page(output: &mut String, page: &PagePlan) {
    output.push_str("{\"column_decisions\":[");
    for (index, decision) in page.column_decisions.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, decision.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&decision.column_index.to_string());
        output.push_str(",\"container\":");
        output.push_str(&decision.container.get().to_string());
        output.push('}');
    }
    output.push_str("],\"float_decisions\":[");
    for (index, decision) in page.float_decisions.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, decision.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&decision.column_index.to_string());
        output.push_str(",\"frame_kind\":");
        push_jcs_string(output, frame_kind_name(decision.frame_kind));
        output.push_str(",\"owner\":");
        output.push_str(&decision.owner.get().to_string());
        output.push_str(",\"owner_local_ordinal\":");
        output.push_str(&decision.owner_local_ordinal.to_string());
        output.push('}');
    }
    output.push_str("],\"footnote_ids\":[");
    for (index, footnote) in page.footnote_ids.iter().enumerate() {
        comma(output, index);
        push_jcs_string(output, footnote.as_str());
    }
    output.push_str("],\"fragments\":[");
    for (index, fragment) in page.fragments.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, fragment.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&fragment.column_index.to_string());
        output.push_str(",\"end\":");
        encode_flow_position(output, &fragment.end);
        output.push_str(",\"frame_kind\":");
        push_jcs_string(output, frame_kind_name(fragment.frame_kind));
        output.push_str(",\"owner\":");
        output.push_str(&fragment.owner.get().to_string());
        output.push_str(",\"owner_local_ordinal\":");
        output.push_str(&fragment.owner_local_ordinal.to_string());
        output.push_str(",\"start\":");
        encode_flow_position(output, &fragment.start);
        output.push('}');
    }
    output.push_str("],\"frames\":[");
    for (index, frame) in page.frames.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, frame.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&frame.column_index.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(output, frame_kind_name(frame.kind));
        output.push('}');
    }
    output.push_str("],\"master_id\":");
    push_jcs_string(output, page.master_id.as_str());
    output.push_str(",\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push_str(",\"resolved_references\":[");
    for (index, reference) in page.resolved_references.iter().enumerate() {
        comma(output, index);
        let provenance = reference.provenance;
        let span = provenance.text_span().range();
        output.push_str("{\"anchor_id\":");
        push_jcs_string(output, reference.anchor_id.as_str());
        output.push_str(",\"buffer_key\":");
        encode_generated_key(output, provenance.buffer_key());
        output.push_str(",\"end_byte\":");
        output.push_str(&span.end_byte().get().to_string());
        output.push_str(",\"start_byte\":");
        output.push_str(&span.start_byte().get().to_string());
        output.push_str(",\"utf8\":");
        push_jcs_string(output, &reference.utf8);
        output.push('}');
    }
    output.push_str("]}");
}

fn encode_placed_anchor(output: &mut String, anchor: &PlacedAnchor) {
    output.push_str("{\"anchor_id\":");
    push_jcs_string(output, anchor.anchor_id.as_str());
    output.push_str(",\"column_index\":");
    output.push_str(&anchor.column_index.to_string());
    output.push_str(",\"frame_kind\":");
    push_jcs_string(output, frame_kind_name(anchor.frame_kind));
    output.push_str(",\"owner\":");
    output.push_str(&anchor.owner_node.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&anchor.page_index.to_string());
    output.push_str(",\"position_in_frame\":{\"x\":");
    output.push_str(&anchor.position_in_frame.x.raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&anchor.position_in_frame.y.raw().to_string());
    output.push_str("}}");
}

fn comma(output: &mut String, index: usize) {
    if index > 0 {
        output.push(',');
    }
}

fn encode_rect(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

fn encode_flow_position(output: &mut String, position: &FlowPosition) {
    output.push_str("{\"block_child_path\":[");
    for (index, child) in position.block_child_path().iter().enumerate() {
        comma(output, index);
        output.push_str(&child.to_string());
    }
    output.push_str("],\"epoch\":");
    encode_layout_epoch(output, position.epoch());
    output.push_str(",\"global_flow_ordinal\":");
    output.push_str(&position.global_flow_ordinal().to_string());
    output.push_str(",\"owner\":");
    output.push_str(&position.owner().get().to_string());
    output.push_str(",\"owner_local_boundary\":");
    output.push_str(&position.owner_local_boundary().to_string());
    output.push('}');
}

fn encode_flow_positions(output: &mut String, positions: &[FlowPosition]) {
    for (index, position) in positions.iter().enumerate() {
        comma(output, index);
        encode_flow_position(output, position);
    }
}

fn encode_layout_epoch(output: &mut String, epoch: LayoutEpoch) {
    output.push_str("{\"admitted_resources_sha256\":");
    push_hex(output, epoch.admitted_resources().bytes());
    output.push_str(",\"document_sha256\":");
    push_hex(output, epoch.document().bytes());
    output.push_str(",\"resolved_input_sha256\":");
    push_hex(output, epoch.references().bytes());
    output.push_str(",\"style_page_master_sha256\":");
    push_hex(output, epoch.style().bytes());
    output.push('}');
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

fn encode_generated_key(output: &mut String, key: GeneratedBufferKey) {
    push_generated_buffer_key_jcs(output, key);
}

const fn frame_kind_name(kind: PageFrameKind) -> &'static str {
    match kind {
        PageFrameKind::Body => "body",
        PageFrameKind::Header => "header",
        PageFrameKind::Footer => "footer",
        PageFrameKind::Footnote => "footnote",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CostComponents {
    keep: i64,
    widow_orphan: i64,
    heading_isolation: i64,
    table_split: i64,
    footnote_split: i64,
    unused_space: i64,
    overflow: i64,
}
impl CostComponents {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        keep: i64,
        widow_orphan: i64,
        heading_isolation: i64,
        table_split: i64,
        footnote_split: i64,
        unused_space: i64,
        overflow: i64,
    ) -> Option<Self> {
        let values = [
            keep,
            widow_orphan,
            heading_isolation,
            table_split,
            footnote_split,
            unused_space,
            overflow,
        ];
        let mut index = 0;
        let mut total = 0i64;
        while index < values.len() {
            let value = values[index];
            if value < -JSON_SAFE_INTEGER_MAX || value > JSON_SAFE_INTEGER_MAX {
                return None;
            }
            total = match total.checked_add(value) {
                Some(total) => total,
                None => return None,
            };
            if total < -JSON_SAFE_INTEGER_MAX || total > JSON_SAFE_INTEGER_MAX {
                return None;
            }
            index += 1;
        }
        Some(Self {
            keep,
            widow_orphan,
            heading_isolation,
            table_split,
            footnote_split,
            unused_space,
            overflow,
        })
    }
    pub const fn keep(self) -> i64 {
        self.keep
    }
    pub const fn widow_orphan(self) -> i64 {
        self.widow_orphan
    }
    pub const fn heading_isolation(self) -> i64 {
        self.heading_isolation
    }
    pub const fn table_split(self) -> i64 {
        self.table_split
    }
    pub const fn footnote_split(self) -> i64 {
        self.footnote_split
    }
    pub const fn unused_space(self) -> i64 {
        self.unused_space
    }
    pub const fn overflow(self) -> i64 {
        self.overflow
    }
    pub const fn total(self) -> i64 {
        self.keep
            + self.widow_orphan
            + self.heading_isolation
            + self.table_split
            + self.footnote_split
            + self.unused_space
            + self.overflow
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FallbackScore {
    hard_violations: u32,
    components: CostComponents,
    total_cost: i64,
}
impl FallbackScore {
    const fn new(hard_violations: u32, components: CostComponents) -> Self {
        Self {
            hard_violations,
            components,
            total_cost: components.total(),
        }
    }
    pub const fn hard_violations(self) -> u32 {
        self.hard_violations
    }
    pub const fn total_cost(self) -> i64 {
        self.total_cost
    }
    pub const fn components(self) -> CostComponents {
        self.components
    }
}

/// State 0 is the unmaterialized seed; pass i produces materialized state i + 1.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutStateIndex(u16);
impl LayoutStateIndex {
    pub const INITIAL: Self = Self(0);
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MaterializedStateIndex(NonZeroU16);
impl MaterializedStateIndex {
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
    pub const fn get(self) -> u16 {
        self.0.get()
    }
    pub const fn pass_index(self) -> u16 {
        self.0.get() - 1
    }
    fn from_pass_index(pass_index: u16) -> Option<Self> {
        match pass_index.checked_add(1) {
            Some(state) => Self::new(state),
            None => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPass {
    pass_index: u16,
    materialized_state: MaterializedStateIndex,
    input_fingerprint: LayoutStateFingerprint,
    output_fingerprint: LayoutStateFingerprint,
    fingerprint_record: PaginationFingerprintRecord,
    fallback_score: FallbackScore,
    materialization: PassMaterializationReceipt,
}
impl LayoutPass {
    pub fn new(
        materialization: PassMaterializationReceipt,
        input_fingerprint: LayoutStateFingerprint,
        flow: &FlowTree,
        pages: Vec<PagePlan>,
        generated_text: GeneratedTextStore,
    ) -> Result<Self, PaginationError> {
        if materialization.summary.input_fingerprint != input_fingerprint
            || materialization.summary.layout_epoch != flow.epoch()
            || materialization.summary.generated_text != generated_text
            || materialization.summary.pages.len() != pages.len()
            || !validate_materialization_chain(flow, &materialization.summary.pages)
            || !validate_materialized_anchors(flow, &materialization.summary.pages)
            || materialization
                .summary
                .pages
                .iter()
                .zip(&pages)
                .any(|(summary, page)| !summary.matches(page))
            || materialization.summary.pages.iter().any(|summary| {
                summary.page_start.epoch() != flow.epoch()
                    || !flow.contains_position(&summary.page_start)
                    || summary.page_start.owner() != summary.flow_owner
                    || !flow.contains_owner(summary.content_owner)
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let pass_index = materialization.summary.pass_index;
        let fallback_score = materialization.summary.fallback_score;
        let placed_anchors = materialization
            .summary
            .pages
            .iter()
            .flat_map(|page| page.placed_anchors.iter().cloned())
            .collect();
        let fingerprint_record =
            PaginationFingerprintRecord::new(flow, pages, generated_text, placed_anchors)?;
        let output_fingerprint = fingerprint_record.fingerprint();
        let materialized_state = MaterializedStateIndex::from_pass_index(pass_index)
            .ok_or(PaginationError::PassIndexOverflow)?;
        Ok(Self {
            pass_index,
            materialized_state,
            input_fingerprint,
            output_fingerprint,
            fingerprint_record,
            fallback_score,
            materialization,
        })
    }
    pub const fn pass_index(&self) -> u16 {
        self.pass_index
    }
    pub const fn materialized_state(&self) -> MaterializedStateIndex {
        self.materialized_state
    }
    pub const fn input_fingerprint(&self) -> LayoutStateFingerprint {
        self.input_fingerprint
    }
    pub const fn output_fingerprint(&self) -> LayoutStateFingerprint {
        self.output_fingerprint
    }
    pub fn pages(&self) -> &[PagePlan] {
        self.fingerprint_record.pages()
    }
    pub const fn fingerprint_record(&self) -> &PaginationFingerprintRecord {
        &self.fingerprint_record
    }
    pub const fn fallback_score(&self) -> FallbackScore {
        self.fallback_score
    }
    pub const fn generated_text(&self) -> &GeneratedTextStore {
        self.fingerprint_record.generated_text()
    }
    pub const fn materialization(&self) -> &PassMaterializationReceipt {
        &self.materialization
    }
    pub fn placed_anchors(&self) -> impl Iterator<Item = &PlacedAnchor> {
        self.materialization
            .summary
            .pages
            .iter()
            .flat_map(|page| page.placed_anchors.iter())
    }

    /// Derives the exact generated-text overlay used by the next pass from
    /// this materialized state. Generated reference transitions are not yet
    /// implemented by the reference backend, so only the canonical zero-site
    /// case can issue a receipt; accepting a caller-provided replacement store
    /// here would break the pagination fingerprint chain.
    pub fn transition_references<'a>(
        &'a self,
        package: &ValidatedParsedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<ReferenceTransitionReceipt<'a>, PaginationError> {
        if package.document_nodes().generated_sites().len() != 0 {
            return Err(PaginationError::UnsupportedReferenceTransition);
        }
        let generated = package
            .bind_generated_text(self.generated_text(), limits)
            .map_err(|_| PaginationError::PackageEpochMismatch)?;
        let working_epoch = self
            .fingerprint_record
            .layout_epoch
            .with_generated_text(generated)
            .map_err(|_| PaginationError::PackageEpochMismatch)?;
        Ok(ReferenceTransitionReceipt {
            session: self.materialization.summary.session.clone(),
            previous_state: self.materialized_state,
            previous_fingerprint: self.output_fingerprint,
            working_epoch,
            generated_text: self.generated_text(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConvergenceStatus {
    Converged,
    CycleFallback {
        cycle_start_state: MaterializedStateIndex,
    },
    MaxPassFallback,
}

/// Selected page/master dimensions captured while the validated master set is
/// still in scope. Downstream crates consume this receipt instead of reopening
/// page-master selection from presentation data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPageGeometry {
    page_index: u32,
    master_id: MasterId,
    width: PositiveLength,
    height: PositiveLength,
}
impl SelectedPageGeometry {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationResult {
    passes: Vec<LayoutPass>,
    status: ConvergenceStatus,
    selected_state: MaterializedStateIndex,
    selected_page_geometry: Vec<SelectedPageGeometry>,
    work_receipt: PaginationBudgetReceipt,
}
impl PaginationResult {
    fn new(
        passes: Vec<LayoutPass>,
        status: ConvergenceStatus,
        input: &PaginationInput<'_>,
        work_receipt: PaginationBudgetReceipt,
    ) -> Result<Self, PaginationError> {
        let options = input.options();
        if options.max_pages == 0
            || options.max_layout_passes == 0
            || options.max_page_break_lookback == 0
        {
            return Err(PaginationError::InvalidOptions);
        }
        if passes.len() > usize::from(options.max_layout_passes)
            || passes
                .iter()
                .any(|pass| pass.pages().len() > options.max_pages as usize)
        {
            return Err(PaginationError::ResourceLimit);
        }
        for pass in &passes {
            let fragments = pass.pages().iter().try_fold(0u64, |count, page| {
                count.checked_add(page.fragments.len() as u64)
            });
            if !matches!(fragments, Some(count) if count <= options.max_fragments) {
                return Err(PaginationError::ResourceLimit);
            }
        }
        if matches!(status, ConvergenceStatus::MaxPassFallback)
            && passes.len() != usize::from(options.max_layout_passes)
        {
            return Err(PaginationError::InvalidMaxPassState);
        }
        validate_work_receipt(&passes, input, &work_receipt)?;
        let known_masters: BTreeMap<&MasterId, _> = input
            .page_masters()
            .masters
            .iter()
            .map(|master| (&master.master_id, master))
            .collect();
        for page in passes.iter().flat_map(|pass| pass.pages()) {
            let master = known_masters
                .get(&page.master_id)
                .ok_or(PaginationError::UnknownPageMaster)?;
            for frame in &page.frames {
                let region = match frame.kind {
                    PageFrameKind::Body => Some(master.body),
                    PageFrameKind::Header => master.header,
                    PageFrameKind::Footer => master.footer,
                    PageFrameKind::Footnote => master.footnote,
                }
                .ok_or(PaginationError::MissingMasterFrame)?;
                if !rect_contains(region, frame.bounds) {
                    return Err(PaginationError::FrameOutsideMaster);
                }
            }
        }
        validate_pass_chain(&passes, input.initial_fingerprint())?;
        validate_terminal_status(&passes, &status, options)?;
        let selected_state = match &status {
            ConvergenceStatus::Converged => {
                let last = passes.last().ok_or(PaginationError::EmptyPasses)?;
                last.materialized_state()
            }
            ConvergenceStatus::CycleFallback { .. } => {
                if options.strict {
                    return Err(PaginationError::FallbackRejectedByStrict);
                }
                select_fallback_state(&passes)?
            }
            ConvergenceStatus::MaxPassFallback => {
                if options.strict {
                    return Err(PaginationError::FallbackRejectedByStrict);
                }
                select_fallback_state(&passes)?
            }
        };
        let selected_page_geometry = passes[usize::from(selected_state.pass_index())]
            .pages()
            .iter()
            .map(|page| {
                let master = known_masters
                    .get(&page.master_id)
                    .ok_or(PaginationError::UnknownPageMaster)?;
                Ok(SelectedPageGeometry {
                    page_index: page.page_index,
                    master_id: page.master_id.clone(),
                    width: master.width,
                    height: master.height,
                })
            })
            .collect::<Result<Vec<_>, PaginationError>>()?;
        Ok(Self {
            passes,
            status,
            selected_state,
            selected_page_geometry,
            work_receipt,
        })
    }

    pub fn passes(&self) -> &[LayoutPass] {
        &self.passes
    }
    pub const fn status(&self) -> &ConvergenceStatus {
        &self.status
    }
    pub const fn selected_state(&self) -> MaterializedStateIndex {
        self.selected_state
    }
    pub fn selected_pass(&self) -> &LayoutPass {
        &self.passes[usize::from(self.selected_state.pass_index())]
    }
    pub fn selected_pages(&self) -> &[PagePlan] {
        self.selected_pass().pages()
    }
    pub fn selected_anchors(&self) -> impl Iterator<Item = &PlacedAnchor> {
        self.selected_pass().placed_anchors()
    }
    pub fn selected_page_geometry(&self) -> &[SelectedPageGeometry] {
        &self.selected_page_geometry
    }
    pub fn final_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_pass().output_fingerprint()
    }
    pub const fn fallback_policy(&self) -> FallbackPolicy {
        FallbackPolicy::LowestCostThenEarliest
    }
    pub const fn work_receipt(&self) -> &PaginationBudgetReceipt {
        &self.work_receipt
    }
}

fn validate_work_receipt(
    passes: &[LayoutPass],
    input: &PaginationInput<'_>,
    receipt: &PaginationBudgetReceipt,
) -> Result<(), PaginationError> {
    if !receipt.session.same_as(&input.session)
        || receipt.initial_fingerprint != input.initial_fingerprint()
        || receipt.layout_epoch != input.initial_state().layout_epoch()
        || receipt.passes.len() != passes.len()
        || receipt
            .passes
            .iter()
            .zip(passes)
            .any(|(summary, pass)| summary != &pass.materialization().summary)
    {
        return Err(PaginationError::InvalidWorkPermit);
    }
    Ok(())
}

fn validate_page_frames_for_context(
    page: &PageContext,
    frames: &[PageFramePlan],
) -> Result<(), PaginationError> {
    if !matches!(
        frames.first(),
        Some(frame) if frame.kind == PageFrameKind::Body && frame.column_index == 0
    ) {
        return Err(PaginationError::MissingBodyFrame);
    }

    let master = page.selected_master();
    let mut previous: Option<(PageFrameKind, u32)> = None;
    for frame in frames {
        validate_rect(frame.bounds)?;
        let expected_column = match previous {
            Some((kind, column)) if kind == frame.kind => column
                .checked_add(1)
                .ok_or(PaginationError::InvalidFrameColumn)?,
            Some((kind, _)) if kind < frame.kind => 0,
            None => 0,
            Some(_) => return Err(PaginationError::InvalidFrameColumn),
        };
        if frame.column_index != expected_column {
            return Err(PaginationError::InvalidFrameColumn);
        }
        let region = match frame.kind {
            PageFrameKind::Body => Some(master.body),
            PageFrameKind::Header => master.header,
            PageFrameKind::Footer => master.footer,
            PageFrameKind::Footnote => master.footnote,
        }
        .ok_or(PaginationError::MissingMasterFrame)?;
        if !rect_contains(region, frame.bounds) {
            return Err(PaginationError::FrameOutsideMaster);
        }
        previous = Some((frame.kind, frame.column_index));
    }
    Ok(())
}

fn rect_contains(outer: Rect, inner: Rect) -> bool {
    let Some(outer_right) = outer.x().raw().checked_add(outer.width().get().raw()) else {
        return false;
    };
    let Some(outer_bottom) = outer.y().raw().checked_add(outer.height().get().raw()) else {
        return false;
    };
    let Some(inner_right) = inner.x().raw().checked_add(inner.width().get().raw()) else {
        return false;
    };
    let Some(inner_bottom) = inner.y().raw().checked_add(inner.height().get().raw()) else {
        return false;
    };
    inner.x().raw() >= outer.x().raw()
        && inner.y().raw() >= outer.y().raw()
        && inner_right <= outer_right
        && inner_bottom <= outer_bottom
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationOutcome {
    result: PaginationResult,
    diagnostics: Vec<AdvisoryDiagnostic>,
}
impl PaginationOutcome {
    pub fn new(
        passes: Vec<LayoutPass>,
        status: ConvergenceStatus,
        input: &PaginationInput<'_>,
        work_receipt: PaginationBudgetReceipt,
    ) -> Result<Self, PaginationError> {
        let fallback = !matches!(status, ConvergenceStatus::Converged);
        let result = PaginationResult::new(passes, status, input, work_receipt)?;
        let diagnostics = if fallback {
            let code =
                DiagnosticCode::new("G6001").ok_or(PaginationError::InvalidFallbackDiagnostic)?;
            let diagnostic = Diagnostic::new(
                code,
                Severity::Warning,
                "pagination selected a materialized fallback state",
            )
            .ok_or(PaginationError::InvalidFallbackDiagnostic)?;
            vec![AdvisoryDiagnostic::new(diagnostic)
                .map_err(|_| PaginationError::InvalidFallbackDiagnostic)?]
        } else {
            vec![]
        };
        Ok(Self {
            result,
            diagnostics,
        })
    }
    pub const fn result(&self) -> &PaginationResult {
        &self.result
    }
    pub fn diagnostics(&self) -> &[AdvisoryDiagnostic] {
        &self.diagnostics
    }
    pub fn into_result(self) -> PaginationResult {
        self.result
    }
}

fn validate_pass_chain(
    passes: &[LayoutPass],
    initial_fingerprint: LayoutStateFingerprint,
) -> Result<(), PaginationError> {
    if passes.is_empty() {
        return Err(PaginationError::EmptyPasses);
    }
    for (expected_index, pass) in passes.iter().enumerate() {
        let expected_index =
            u16::try_from(expected_index).map_err(|_| PaginationError::PassIndexOverflow)?;
        if pass.pass_index() != expected_index {
            return Err(PaginationError::InvalidPassIndex);
        }
        if expected_index == 0 && pass.input_fingerprint() != initial_fingerprint {
            return Err(PaginationError::InitialFingerprintMismatch);
        }
        if let Some(previous) = expected_index
            .checked_sub(1)
            .and_then(|index| passes.get(usize::from(index)))
        {
            if pass.input_fingerprint() != previous.output_fingerprint() {
                return Err(PaginationError::BrokenFingerprintChain);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedTermination {
    Stable,
    Cycle(MaterializedStateIndex),
}

fn observed_termination(passes: &[LayoutPass]) -> Option<ObservedTermination> {
    let last = passes.last()?;
    if last.output_fingerprint() == last.input_fingerprint() {
        return Some(ObservedTermination::Stable);
    }
    let repeated = last.output_fingerprint();
    passes
        .iter()
        .take(passes.len() - 1)
        .find(|pass| pass.output_fingerprint() == repeated)
        .map(|pass| ObservedTermination::Cycle(pass.materialized_state()))
}

fn validate_terminal_status(
    passes: &[LayoutPass],
    status: &ConvergenceStatus,
    options: PaginationOptions,
) -> Result<(), PaginationError> {
    for prefix_len in 1..passes.len() {
        if observed_termination(&passes[..prefix_len]).is_some() {
            return Err(PaginationError::PassesContinueAfterTermination);
        }
    }
    let observed = observed_termination(passes);
    match (status, observed) {
        (ConvergenceStatus::Converged, Some(ObservedTermination::Stable)) => Ok(()),
        (
            ConvergenceStatus::CycleFallback { cycle_start_state },
            Some(ObservedTermination::Cycle(observed_start)),
        ) if *cycle_start_state == observed_start => Ok(()),
        (ConvergenceStatus::MaxPassFallback, None)
            if passes.len() == usize::from(options.max_layout_passes) =>
        {
            Ok(())
        }
        (ConvergenceStatus::Converged, _) => Err(PaginationError::InvalidConvergenceStatus),
        (ConvergenceStatus::CycleFallback { .. }, _) => Err(PaginationError::InvalidCycleState),
        (ConvergenceStatus::MaxPassFallback, _) => Err(PaginationError::InvalidMaxPassState),
    }
}

fn select_fallback_state(passes: &[LayoutPass]) -> Result<MaterializedStateIndex, PaginationError> {
    passes
        .iter()
        .min_by_key(|pass| {
            (
                pass.fallback_score().hard_violations(),
                pass.fallback_score().total_cost(),
                pass.pages().len(),
                pass.materialized_state(),
            )
        })
        .map(LayoutPass::materialized_state)
        .ok_or(PaginationError::EmptyPasses)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaginationError {
    MissingDefaultMaster,
    PageLimit,
    NoProgress,
    ArithmeticOverflow,
    ResourceLimit,
    FatalLayout,
    InvalidOptions,
    InvalidPreparedLayout,
    EmptyPasses,
    PassIndexOverflow,
    InvalidPassIndex,
    BrokenFingerprintChain,
    InvalidConvergenceStatus,
    InvalidCycleState,
    InvalidMaxPassState,
    FallbackRejectedByStrict,
    EmptyPages,
    InvalidPageIndex,
    UnknownPageMaster,
    DuplicateResolvedReference,
    DuplicateFootnote,
    InvalidFootnoteClosure,
    DuplicateFragmentPosition,
    MissingBodyFrame,
    InvalidFrameColumn,
    InvalidFragmentRange,
    DuplicateFloatDecision,
    InvalidColumnDecision,
    FingerprintIntegerOutOfRange,
    InitialFingerprintMismatch,
    PassesContinueAfterTermination,
    InvalidFallbackDiagnostic,
    EpochMismatch,
    MissingMasterFrame,
    FrameOutsideMaster,
    UnknownFrameReference,
    FragmentOutsideFrame,
    UnknownFlowOwner,
    WorkBudgetAlreadyIssued,
    InvalidWorkPermit,
    PackageEpochMismatch,
    UnsupportedReferenceTransition,
    InvalidInitialReferenceSeed,
}

pub trait Paginator {
    fn paginate<F: Fragmenter, P: LayoutPassProvider>(
        &self,
        input: PaginationInput<'_>,
        pass_provider: &P,
        fragmenter: &F,
    ) -> Result<PaginationOutcome, PaginationError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{
        Length, NodeId, NonNegativeLength, PortablePath, PositiveLength, ResourceLimits, SourceId,
        ValidatedResourceLimits,
    };
    use typaxis_document::ValidatedDocumentNodeIndex;
    use typaxis_layout::{CanonicalFlowIrBuilder, FragmentDraft, FragmentResult, LayoutEpoch};
    use typaxis_linebreak::ValidatedParagraphItemRegistry;
    use typaxis_resource_admission::AdmittedResourceResolver;
    use typaxis_syntax::{
        PackagePaginationContext, PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser,
        SourceFile, ValidatedParsedPackage,
    };
    use typaxis_text::TextStore;

    fn validated_package_with_uri(uri: &str) -> ValidatedParsedPackage {
        parsed_reference_package(uri, "")
    }
    fn parsed_reference_package(uri: &str, text: &str) -> ValidatedParsedPackage {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(uri).unwrap(),
            text: text.to_owned(),
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        match ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) {
            ParseOutcome::Parsed { package, .. } => *package,
            ParseOutcome::Failed { failure } => panic!("reference parse failed: {failure:?}"),
        }
    }
    fn validated_flow_package() -> ValidatedParsedPackage {
        parsed_reference_package("flow-input.tsf", "anchor:chapter\nparagraph")
    }
    fn validated_package() -> ValidatedParsedPackage {
        validated_package_with_uri("input.tsf")
    }
    fn pagination_context() -> PackagePaginationContext {
        validated_package().pagination_context()
    }
    fn epoch_for(package: &ValidatedParsedPackage, store: &GeneratedTextStore) -> LayoutEpoch {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = package.bind_generated_text(store, &limits).unwrap();
        LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap()
    }
    fn initial_state() -> InitialPaginationState {
        let package = validated_package();
        let store = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &store)).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        InitialPaginationState::new(&flow, &package, &limits).unwrap()
    }
    fn transitioned_input(pass: &LayoutPass) -> LayoutPassInput<'_> {
        let package = validated_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        LayoutPassInput::transitioned(pass.transition_references(&package, &limits).unwrap())
    }
    fn fingerprint(value: u8) -> LayoutStateFingerprint {
        if value == 0 {
            initial_state().fingerprint()
        } else {
            LayoutStateFingerprint::from_untrusted_bytes([value; 32])
        }
    }
    fn generated_store() -> GeneratedTextStore {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let parsed = TextStore::new(vec![]).unwrap();
        GeneratedTextStore::new(
            vec![],
            &ValidatedDocumentNodeIndex::empty_document(),
            &limits,
            &parsed,
        )
        .unwrap()
    }
    fn page(page_index: u32, marker: &str) -> PagePlan {
        let size = PositiveLength::new(Length::from_raw(10).unwrap()).unwrap();
        let inset_raw = i64::from(marker.as_bytes().first().copied().unwrap_or(0) % 10);
        let inset = Length::from_raw(inset_raw).unwrap();
        let width = PositiveLength::new(Length::from_raw(10 - inset_raw).unwrap()).unwrap();
        PagePlan {
            page_index,
            master_id: MasterId::new("default").unwrap(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: Rect::new(inset, Length::ZERO, width, size),
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        }
    }
    fn pass(
        budget: &mut PaginationWorkBudget,
        pass_input: LayoutPassInput<'_>,
        score: FallbackScore,
        markers: &[&str],
    ) -> LayoutPass {
        let package = validated_package();
        let generated = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &generated)).unwrap();
        let pages: Vec<_> = markers
            .iter()
            .enumerate()
            .map(|(index, marker)| page(index as u32, marker))
            .collect();
        let pass_index = pass_input.state_index().get();
        let input_fingerprint = pass_input.fingerprint();
        let mut permit = budget.begin_pass(pass_index, pass_input).unwrap();
        for page in &pages {
            let cursor = FlowCursor::document_start(&flow);
            let selection =
                typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
            let context =
                PageContext::select(page.page_index, &selection, &package.pagination_context())
                    .unwrap();
            permit.begin_page(&context, &cursor, &page.frames).unwrap();
            FragmentWorkBudget::consume_fragments(
                &mut permit,
                u64::try_from(page.fragments.len()).unwrap(),
            )
            .unwrap();
            permit.finish_page(page).unwrap();
        }
        for _ in 0..score.hard_violations() {
            permit.record_hard_violation().unwrap();
        }
        let components = score.components();
        permit.add_keep_cost(components.keep()).unwrap();
        permit
            .add_widow_orphan_cost(components.widow_orphan())
            .unwrap();
        permit
            .add_heading_isolation_cost(components.heading_isolation())
            .unwrap();
        permit
            .add_table_split_cost(components.table_split())
            .unwrap();
        permit
            .add_footnote_split_cost(components.footnote_split())
            .unwrap();
        permit
            .add_unused_space_cost(components.unused_space())
            .unwrap();
        permit.add_overflow_cost(components.overflow()).unwrap();
        let receipt = permit.finish(&flow, &pages).unwrap();
        LayoutPass::new(receipt, input_fingerprint, &flow, pages, generated).unwrap()
    }

    fn score(hard_violations: u32, keep: i64) -> FallbackScore {
        FallbackScore::new(
            hard_violations,
            CostComponents::new(keep, 0, 0, 0, 0, 0, 0).unwrap(),
        )
    }

    fn input<'a>(
        initial: LayoutStateFingerprint,
        options: PaginationOptions,
        package_context: &'a PackagePaginationContext,
    ) -> PaginationInput<'a> {
        let state = initial_state();
        assert_eq!(initial, state.fingerprint());
        PaginationInput::new(state, package_context, options).unwrap()
    }
    fn blank_page_context(
        page_index: u32,
        package_context: &PackagePaginationContext,
    ) -> (PageContext, FlowCursor, FlowTree) {
        let package = validated_package();
        let generated = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &generated)).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page = PageContext::select(page_index, &selection, package_context).unwrap();
        (page, cursor, flow)
    }
    fn options(max_layout_passes: u16, strict: bool) -> PaginationOptions {
        let limits = ResourceLimits {
            max_layout_passes,
            ..ResourceLimits::default()
        };
        let limits = ValidatedResourceLimits::new(limits).unwrap();
        PaginationOptions::from_limits(&limits, strict)
    }
    fn default_options() -> PaginationOptions {
        options(ResourceLimits::default().max_layout_passes, false)
    }

    struct OneFragmenter {
        fragment: FragmentDraft,
        terminal: FlowCursor,
        discovered_anchors: Vec<DiscoveredAnchor>,
    }
    impl Fragmenter for OneFragmenter {
        fn fragment(
            &self,
            _request: &FragmentRequest<'_>,
            budget: &mut dyn FragmentWorkBudget,
        ) -> Result<FragmentResult, FragmentError> {
            budget.consume_fragments(1)?;
            Ok(FragmentResult {
                fragments: vec![self.fragment.clone()],
                continuation: Continuation::Exhausted(Box::new(self.terminal.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: self.discovered_anchors.clone(),
            })
        }
    }

    struct EmptyMoreFragmenter {
        next: FlowCursor,
    }
    impl Fragmenter for EmptyMoreFragmenter {
        fn fragment(
            &self,
            _request: &FragmentRequest<'_>,
            _budget: &mut dyn FragmentWorkBudget,
        ) -> Result<FragmentResult, FragmentError> {
            Ok(FragmentResult {
                fragments: vec![],
                continuation: Continuation::More(Box::new(self.next.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            })
        }
    }

    #[test]
    fn materialized_state_excludes_state_zero() {
        assert!(MaterializedStateIndex::new(0).is_none());
        assert_eq!(MaterializedStateIndex::new(1).unwrap().pass_index(), 0);
    }

    #[test]
    fn reference_transition_is_issued_by_the_exact_package_and_predecessor() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), options(2, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let foreign = validated_package_with_uri("foreign-input.tsf");
        assert_eq!(
            first.transition_references(&foreign, &limits),
            Err(PaginationError::PackageEpochMismatch)
        );

        let package = validated_package();
        let next =
            LayoutPassInput::transitioned(first.transition_references(&package, &limits).unwrap());
        assert_eq!(next.state_index().get(), first.materialized_state().get());
        assert_eq!(next.fingerprint(), first.output_fingerprint());
        assert!(next
            .layout_epoch()
            .same_stable_inputs(first.fingerprint_record().layout_epoch()));
        assert_eq!(next.generated_text(), first.generated_text());
    }

    #[test]
    fn pass_work_rejects_an_identical_predecessor_from_another_session() {
        let masters = pagination_context();
        let mut input_a = input(fingerprint(0), options(2, false), &masters);
        let mut input_b = input(fingerprint(0), options(2, false), &masters);
        let mut budget_a = input_a.take_work_budget().unwrap();
        let mut budget_b = input_b.take_work_budget().unwrap();
        let first_a = pass(
            &mut budget_a,
            LayoutPassInput::initial(&input_a),
            score(0, 0),
            &["same"],
        );
        let first_b = pass(
            &mut budget_b,
            LayoutPassInput::initial(&input_b),
            score(0, 0),
            &["same"],
        );
        assert_eq!(first_a.output_fingerprint(), first_b.output_fingerprint());

        let package = validated_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let foreign = LayoutPassInput::transitioned(
            first_a.transition_references(&package, &limits).unwrap(),
        );
        assert!(matches!(
            budget_b.begin_pass(1, foreign),
            Err(PaginationError::InvalidWorkPermit)
        ));

        // The rejected foreign capability did not consume pass work. The
        // exact predecessor issued by this budget's own session still starts.
        let own = LayoutPassInput::transitioned(
            first_b.transition_references(&package, &limits).unwrap(),
        );
        assert!(budget_b.begin_pass(1, own).is_ok());
    }

    #[test]
    fn pagination_session_capability_has_opaque_order_independent_debug() {
        let first = PaginationSessionId::issue();
        let second = PaginationSessionId::issue();

        assert_eq!(first, first.clone());
        assert_ne!(first, second);
        assert_eq!(format!("{first:?}"), "PaginationSessionId(<opaque>)");
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    #[test]
    fn placed_anchor_order_is_anchor_id_not_page_order() {
        let point = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let mut anchors = vec![
            PlacedAnchor {
                anchor_id: AnchorId::new("z-last").unwrap(),
                owner_node: NodeId::new(1),
                page_index: 0,
                frame_kind: PageFrameKind::Body,
                column_index: 0,
                position_in_frame: point,
            },
            PlacedAnchor {
                anchor_id: AnchorId::new("a-first").unwrap(),
                owner_node: NodeId::new(2),
                page_index: 1,
                frame_kind: PageFrameKind::Body,
                column_index: 0,
                position_in_frame: point,
            },
        ];
        canonicalize_placed_anchors(&mut anchors);
        assert_eq!(anchors[0].anchor_id().as_str(), "a-first");
        assert_eq!(anchors[0].page_index(), 1);
    }

    #[test]
    fn fallback_selects_score_before_earliest_state() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(4, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(1, 1),
            &["a"],
        );
        let second_input = transitioned_input(&first);
        let second = pass(&mut budget, second_input, score(0, 10), &["b"]);
        let third_input = transitioned_input(&second);
        let third = pass(&mut budget, third_input, score(0, 9), &["d"]);
        let fourth_input = transitioned_input(&third);
        let fourth = pass(&mut budget, fourth_input, score(0, 10), &["e"]);
        let result = PaginationResult::new(
            vec![first, second, third, fourth],
            ConvergenceStatus::MaxPassFallback,
            &pagination_input,
            budget.finish(),
        )
        .unwrap();
        assert_eq!(result.selected_state().get(), 3);
        assert_eq!(result.selected_pass().pass_index(), 2);
        assert_eq!(
            result.final_fingerprint(),
            result.selected_pass().output_fingerprint()
        );
    }

    #[test]
    fn strict_mode_rejects_fallback() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(1, true), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let passes = vec![pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        )];
        assert_eq!(
            PaginationResult::new(
                passes,
                ConvergenceStatus::MaxPassFallback,
                &pagination_input,
                budget.finish(),
            ),
            Err(PaginationError::FallbackRejectedByStrict)
        );
    }

    #[test]
    fn converged_result_selects_last_materialized_pass() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(8, true), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 2),
            &["a"],
        );
        let stable_input = transitioned_input(&first);
        let stable = pass(&mut budget, stable_input, score(0, 1), &["a"]);
        let stable_fingerprint = stable.output_fingerprint();
        let result = PaginationResult::new(
            vec![first, stable],
            ConvergenceStatus::Converged,
            &pagination_input,
            budget.finish(),
        )
        .unwrap();
        assert_eq!(result.selected_state().get(), 2);
        assert_eq!(result.final_fingerprint(), stable_fingerprint);
    }

    #[test]
    fn cycle_fallback_can_only_name_a_prior_materialized_state() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(3, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let second_input = transitioned_input(&first);
        let second = pass(&mut budget, second_input, score(0, 0), &["b"]);
        let third_input = transitioned_input(&second);
        let third = pass(&mut budget, third_input, score(0, 0), &["a"]);
        let result = PaginationResult::new(
            vec![first, second, third],
            ConvergenceStatus::CycleFallback {
                cycle_start_state: MaterializedStateIndex::new(1).unwrap(),
            },
            &pagination_input,
            budget.finish(),
        )
        .unwrap();
        assert_eq!(result.selected_state().get(), 1);
        assert!(MaterializedStateIndex::new(0).is_none());
    }

    #[test]
    fn pass_chain_must_be_contiguous() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(2, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let wrong_input = LayoutPassInput {
            session: pagination_input.session.clone(),
            state_index: LayoutStateIndex::new(1),
            fingerprint: fingerprint(9),
            layout_epoch: first.fingerprint_record().layout_epoch(),
            generated_text: first.generated_text(),
        };
        let second = pass(&mut budget, wrong_input, score(0, 0), &["b"]);
        assert_eq!(
            PaginationResult::new(
                vec![first, second],
                ConvergenceStatus::MaxPassFallback,
                &pagination_input,
                budget.finish(),
            ),
            Err(PaginationError::BrokenFingerprintChain)
        );
    }

    #[test]
    fn prepared_layout_rejects_cursor_epoch_mismatch() {
        let state = initial_state();
        let first = state.layout_epoch();
        let first_package = validated_package();
        let generated = generated_store();
        let other_package = validated_package_with_uri("other-input.tsf");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new(&other_package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = other_package
            .bind_generated_text(&generated, &limits)
            .unwrap();
        let second = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let flow = FlowTree::empty(&first_package, first).unwrap();
        let other = FlowTree::empty(&other_package, second).unwrap();
        let cursor = FlowCursor::document_start(&other);
        let masters = pagination_context();
        let pagination_input = PaginationInput::new(state, &masters, default_options()).unwrap();
        assert_eq!(
            PreparedLayout::new(LayoutPassInput::initial(&pagination_input), flow, cursor),
            Err(PaginationError::InvalidPreparedLayout)
        );
    }

    #[test]
    fn pagination_input_rejects_another_packages_page_master_receipt() {
        let state = initial_state();
        let other_context = validated_package_with_uri("other-input.tsf").pagination_context();
        assert_eq!(
            PaginationInput::new(state, &other_context, default_options()),
            Err(PaginationError::PackageEpochMismatch)
        );
    }

    #[test]
    fn fallback_cost_is_a_signed_json_safe_integer() {
        assert_eq!(
            score(0, -JSON_SAFE_INTEGER_MAX).total_cost(),
            -JSON_SAFE_INTEGER_MAX
        );
        assert!(CostComponents::new(JSON_SAFE_INTEGER_MAX + 1, 0, 0, 0, 0, 0, 0).is_none());
        assert!(CostComponents::new(-JSON_SAFE_INTEGER_MAX - 1, 0, 0, 0, 0, 0, 0).is_none());
        assert!(CostComponents::new(JSON_SAFE_INTEGER_MAX, 1, 0, 0, 0, 0, 0).is_none());
    }

    #[test]
    fn work_budget_rejects_each_max_plus_one_before_work() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 1,
            max_layout_passes: 1,
            max_fragments: 1,
            max_page_break_lookback: 1,
            max_footnote_reflows_per_page: 1,
            max_column_balance_candidates: 1,
            max_float_queue: 1,
            max_float_carry_pages: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        let options = PaginationOptions::from_limits(&limits, false);
        let masters = pagination_context();

        let mut pass_limited_input = input(fingerprint(0), options, &masters);
        let mut pass_limited_budget = pass_limited_input.take_work_budget().unwrap();
        let first = pass(
            &mut pass_limited_budget,
            LayoutPassInput::initial(&pass_limited_input),
            score(0, 0),
            &["a"],
        );
        assert!(matches!(
            pass_limited_budget.begin_pass(1, transitioned_input(&first)),
            Err(PaginationError::ResourceLimit)
        ));

        let mut page_limited_input = input(fingerprint(0), options, &masters);
        let mut page_limited_budget = page_limited_input.take_work_budget().unwrap();
        let mut page_permit = page_limited_budget
            .begin_pass(0, LayoutPassInput::initial(&page_limited_input))
            .unwrap();
        let first_page = page(0, "a");
        let (first_context, first_cursor, _first_flow) = blank_page_context(0, &masters);
        page_permit
            .begin_page(&first_context, &first_cursor, &first_page.frames)
            .unwrap();
        page_permit.finish_page(&first_page).unwrap();
        let (second_context, second_cursor, _second_flow) = blank_page_context(1, &masters);
        assert_eq!(
            page_permit.begin_page(&second_context, &second_cursor, &first_page.frames),
            Err(PaginationError::ResourceLimit)
        );

        let mut work_input = input(fingerprint(0), options, &masters);
        let mut work_budget = work_input.take_work_budget().unwrap();
        let mut permit = work_budget
            .begin_pass(0, LayoutPassInput::initial(&work_input))
            .unwrap();
        permit
            .begin_page(&first_context, &first_cursor, &first_page.frames)
            .unwrap();
        assert!(permit.consume_fragments(1).is_ok());
        assert_eq!(
            permit.consume_fragments(1),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.consume_footnote_reflow(0).is_ok());
        assert_eq!(
            permit.consume_footnote_reflow(0),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.consume_column_candidate(NodeId::new(1)).is_ok());
        assert_eq!(
            permit.consume_column_candidate(NodeId::new(1)),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.enqueue_float(NodeId::new(1), 0).is_ok());
        assert_eq!(
            permit.enqueue_float(NodeId::new(2), 0),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.consume_float_carry(NodeId::new(1), 0).is_ok());
        assert_eq!(
            permit.consume_float_carry(NodeId::new(1), 0),
            Err(FragmentError::ResourceLimit)
        );
        assert_eq!(
            permit.dequeue_float(NodeId::new(2), 0),
            Err(FragmentError::InvalidFloatState)
        );
        assert!(permit.dequeue_float(NodeId::new(1), 0).is_ok());
        assert_eq!(
            permit.consume_float_carry(NodeId::new(1), 0),
            Err(FragmentError::InvalidFloatState)
        );
        let mut lookback = permit.begin_page_break_search().unwrap();
        assert!(lookback.consume_candidate().is_ok());
        assert_eq!(
            lookback.consume_candidate(),
            Err(PaginationError::ResourceLimit)
        );
        assert!(permit.finish_page_break_search(lookback).is_ok());
        assert_eq!(
            permit.begin_page_break_search(),
            Err(PaginationError::InvalidWorkPermit)
        );
        assert_eq!(
            permit.finish_page(&first_page),
            Err(PaginationError::InvalidWorkPermit)
        );

        let mut input = PaginationInput::new(initial_state(), &masters, options).unwrap();
        assert!(input.take_work_budget().is_ok());
        assert_eq!(
            input.take_work_budget(),
            Err(PaginationError::WorkBudgetAlreadyIssued)
        );
    }

    #[test]
    fn unresolved_float_queue_cannot_materialize_a_pass() {
        let package_context = pagination_context();
        let mut input = input(fingerprint(0), default_options(), &package_context);
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let plan = page(0, "d");
        let (page_context, cursor, flow) = blank_page_context(0, &package_context);
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();
        permit.enqueue_float(NodeId::new(0), 0).unwrap();
        permit.finish_page(&plan).unwrap();
        assert_eq!(
            permit.finish(&flow, core::slice::from_ref(&plan)),
            Err(PaginationError::InvalidWorkPermit)
        );
    }

    #[test]
    fn page_plan_decisions_require_exact_work_receipts() {
        fn assert_rejected(plan: PagePlan) {
            let package_context = pagination_context();
            let mut input = input(fingerprint(0), default_options(), &package_context);
            let mut budget = input.take_work_budget().unwrap();
            let mut permit = budget
                .begin_pass(0, LayoutPassInput::initial(&input))
                .unwrap();
            let (page_context, cursor, _flow) = blank_page_context(0, &package_context);
            permit
                .begin_page(&page_context, &cursor, &plan.frames)
                .unwrap();
            assert_eq!(
                permit.finish_page(&plan),
                Err(PaginationError::InvalidWorkPermit)
            );
        }

        let mut footnote = page(0, "a");
        footnote.footnote_ids = vec![FootnoteId::new("note").unwrap()];
        assert_rejected(footnote);

        let mut float = page(0, "a");
        float.float_decisions = vec![FloatDecision {
            owner: NodeId::new(0),
            owner_local_ordinal: 0,
            frame_kind: PageFrameKind::Body,
            column_index: 0,
            bounds: float.frames[0].bounds,
        }];
        assert_rejected(float);

        let mut column = page(0, "a");
        column.column_decisions = vec![ColumnDecision {
            container: NodeId::new(0),
            column_index: 0,
            bounds: column.frames[0].bounds,
        }];
        assert_rejected(column);
    }

    #[test]
    fn float_and_column_state_machines_issue_exact_page_receipts() {
        let package_context = pagination_context();
        let mut input = input(fingerprint(0), default_options(), &package_context);
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let mut plan = page(0, "a");
        let (page_context, cursor, flow) = blank_page_context(0, &package_context);
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();

        permit.enqueue_float(NodeId::new(0), 0).unwrap();
        permit.dequeue_float(NodeId::new(0), 0).unwrap();
        let float = FloatDecision {
            owner: NodeId::new(0),
            owner_local_ordinal: 0,
            frame_kind: PageFrameKind::Body,
            column_index: 0,
            bounds: plan.frames[0].bounds,
        };
        permit.record_float_decision(float.clone()).unwrap();
        plan.float_decisions.push(float);

        permit.consume_column_candidate(NodeId::new(0)).unwrap();
        let column = ColumnDecision {
            container: NodeId::new(0),
            column_index: 0,
            bounds: plan.frames[0].bounds,
        };
        permit
            .record_column_decisions(NodeId::new(0), vec![column.clone()])
            .unwrap();
        plan.column_decisions.push(column);

        permit.finish_page(&plan).unwrap();
        assert!(permit.finish(&flow, core::slice::from_ref(&plan)).is_ok());
    }

    #[test]
    fn max_pass_status_requires_exact_limit_and_returns_advisory() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut default_input = input(seed, default_options(), &masters);
        let mut default_budget = default_input.take_work_budget().unwrap();
        let materialized = pass(
            &mut default_budget,
            LayoutPassInput::initial(&default_input),
            score(0, 0),
            &["a"],
        );
        assert_eq!(
            PaginationOutcome::new(
                vec![materialized],
                ConvergenceStatus::MaxPassFallback,
                &default_input,
                default_budget.finish(),
            ),
            Err(PaginationError::InvalidMaxPassState)
        );
        let mut exact_input = input(seed, options(1, false), &masters);
        let mut exact_budget = exact_input.take_work_budget().unwrap();
        let materialized = pass(
            &mut exact_budget,
            LayoutPassInput::initial(&exact_input),
            score(0, 0),
            &["a"],
        );
        let outcome = PaginationOutcome::new(
            vec![materialized],
            ConvergenceStatus::MaxPassFallback,
            &exact_input,
            exact_budget.finish(),
        )
        .unwrap();
        assert_eq!(outcome.diagnostics().len(), 1);
        assert_eq!(outcome.result().selected_state().get(), 1);
    }

    #[test]
    fn materialized_pass_requires_a_nonempty_dense_page_sequence() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), default_options(), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let empty_package = validated_package();
        let empty_store = generated_store();
        let empty_flow =
            FlowTree::empty(&empty_package, epoch_for(&empty_package, &empty_store)).unwrap();
        let empty_permit = budget
            .begin_pass(0, LayoutPassInput::initial(&pagination_input))
            .unwrap();
        assert_eq!(
            empty_permit.finish(&empty_flow, &[]),
            Err(PaginationError::InvalidWorkPermit)
        );
        let mut wrong = page(1, "a");
        wrong.page_index = 1;
        let wrong_package = validated_package();
        let wrong_store = generated_store();
        let wrong_flow =
            FlowTree::empty(&wrong_package, epoch_for(&wrong_package, &wrong_store)).unwrap();
        let forged_receipt = PassMaterializationReceipt {
            summary: PassBudgetSummary {
                session: pagination_input.session.clone(),
                pass_index: 0,
                input_fingerprint: fingerprint(0),
                layout_epoch: wrong_flow.epoch(),
                generated_text: wrong_store.clone(),
                pages: vec![PageBudgetSummary {
                    page_index: 1,
                    page_start: wrong_flow.positions()[0].clone(),
                    flow_owner: NodeId::new(0),
                    content_owner: NodeId::new(0),
                    master_id: wrong.master_id.clone(),
                    frames: wrong.frames.clone(),
                    consumed_fragment_count: 0,
                    fragments: vec![],
                    footnote_ids: BTreeSet::new(),
                    float_decisions: vec![],
                    column_decisions: vec![],
                    resolved_references: vec![],
                    placed_anchors: vec![],
                    continuation: None,
                    fragmenter_invoked: false,
                    next_fragment_cursor: wrong_flow.positions()[0].clone(),
                    fragmentation_exhausted: false,
                    lookback_search_issued: false,
                    lookback_search_completed: false,
                }],
                fallback_score: score(0, 0),
            },
        };
        assert_eq!(
            LayoutPass::new(
                forged_receipt,
                fingerprint(0),
                &wrong_flow,
                vec![wrong],
                wrong_store,
            ),
            Err(PaginationError::InvalidPageIndex)
        );
    }

    #[test]
    fn materialized_pass_is_bound_to_the_permitted_input_identity() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), default_options(), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let package = validated_package();
        let store = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &store)).unwrap();
        let pages = vec![page(0, "a")];
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&pagination_input))
            .unwrap();
        let (context, cursor, _context_flow) = blank_page_context(0, &masters);
        permit
            .begin_page(&context, &cursor, &pages[0].frames)
            .unwrap();
        permit.finish_page(&pages[0]).unwrap();
        let receipt = permit.finish(&flow, &pages).unwrap();
        assert_eq!(
            LayoutPass::new(receipt, fingerprint(9), &flow, pages, store),
            Err(PaginationError::InvalidWorkPermit)
        );
    }

    #[test]
    fn page_materialization_accepts_only_the_fragmenter_recorded_sequence() {
        let package = validated_flow_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(initial, &package_context, default_options()).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let mut plan = page(0, "d");
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();

        let next =
            FlowCursor::at(&flow, 1, typaxis_layout::CursorPosition::ParagraphItem(0)).unwrap();
        let bootstrap_request = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        permit
            .run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &bootstrap_request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();

        let fragment = FragmentDraft::new(
            flow.positions()[1].clone(),
            flow.positions().last().unwrap().clone(),
            plan.frames[0].bounds,
            0,
        )
        .unwrap();
        let fragmenter = OneFragmenter {
            fragment,
            terminal: flow.terminal_cursor(),
            discovered_anchors: vec![DiscoveredAnchor {
                anchor_id: AnchorId::new("chapter").unwrap(),
                owner_node: NodeId::new(2),
                position_in_frame: Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                },
            }],
        };
        let request = FragmentRequest::new(
            &flow,
            &next,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context,
        )
        .unwrap();
        let materialized = permit
            .run_fragmenter(&fragmenter, &request, PageFrameKind::Body, 0)
            .unwrap();
        assert_eq!(materialized.placed_fragments()[0].owner_local_ordinal, 0);
        assert_eq!(materialized.placed_anchors()[0].page_index(), 0);
        assert_eq!(
            materialized.placed_anchors()[0]
                .position_on_page(plan.frames[0].bounds)
                .unwrap(),
            Point {
                x: plan.frames[0].bounds.x(),
                y: plan.frames[0].bounds.y(),
            }
        );
        assert_eq!(
            permit.run_fragmenter(&fragmenter, &request, PageFrameKind::Body, 0),
            Err(FragmentError::InvalidPageContext)
        );
        plan.fragments
            .extend_from_slice(materialized.placed_fragments());
        let mut sparse = plan.clone();
        sparse.fragments[0].owner_local_ordinal = 1;
        assert_eq!(
            PaginationFingerprintRecord::new(
                &flow,
                vec![sparse],
                generated.clone(),
                materialized.placed_anchors().to_vec(),
            ),
            Err(PaginationError::InvalidFragmentRange)
        );
        assert!(permit.finish_page(&plan).is_ok());
        let receipt = permit.finish(&flow, core::slice::from_ref(&plan)).unwrap();
        let mut wrong_anchor = receipt.clone();
        wrong_anchor.summary.pages[0].placed_anchors[0].owner_node = NodeId::new(1);
        assert_eq!(
            LayoutPass::new(
                wrong_anchor,
                input.initial_fingerprint(),
                &flow,
                vec![plan.clone()],
                generated.clone(),
            ),
            Err(PaginationError::InvalidWorkPermit)
        );
        let mut missing_anchor = receipt.clone();
        missing_anchor.summary.pages[0].placed_anchors.clear();
        assert_eq!(
            LayoutPass::new(
                missing_anchor,
                input.initial_fingerprint(),
                &flow,
                vec![plan.clone()],
                generated.clone(),
            ),
            Err(PaginationError::InvalidWorkPermit)
        );
        let mut moved_anchor = receipt.clone();
        moved_anchor.summary.pages[0].placed_anchors[0]
            .position_in_frame
            .x = Length::from_raw(1).unwrap();
        let moved_pass = LayoutPass::new(
            moved_anchor,
            input.initial_fingerprint(),
            &flow,
            vec![plan.clone()],
            generated.clone(),
        )
        .unwrap();
        let pass = LayoutPass::new(
            receipt,
            input.initial_fingerprint(),
            &flow,
            vec![plan],
            generated,
        )
        .unwrap();
        assert_eq!(pass.placed_anchors().count(), 1);
        assert_ne!(pass.output_fingerprint(), moved_pass.output_fingerprint());
    }

    #[test]
    fn pass_receipt_requires_an_exact_continuation_chain_to_terminal() {
        let package = validated_flow_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(initial, &package_context, default_options()).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let plan = page(0, "d");
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();
        let request = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context,
        )
        .unwrap();
        let next =
            FlowCursor::at(&flow, 1, typaxis_layout::CursorPosition::ParagraphItem(0)).unwrap();
        permit
            .run_fragmenter(
                &EmptyMoreFragmenter { next },
                &request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();
        permit.finish_page(&plan).unwrap();

        let wrong_selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let wrong_page = PageContext::select(1, &wrong_selection, &package_context).unwrap();
        assert_eq!(
            permit.begin_page(&wrong_page, &cursor, &plan.frames),
            Err(PaginationError::InvalidWorkPermit)
        );
        assert_eq!(
            permit.finish(&flow, core::slice::from_ref(&plan)),
            Err(PaginationError::InvalidWorkPermit)
        );
    }

    #[test]
    fn multiple_frame_invocations_follow_the_exact_continuation() {
        let package = validated_flow_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let package_context = package.pagination_context();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let mut input = PaginationInput::new(initial, &package_context, default_options()).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();

        let half = PositiveLength::new(Length::from_raw(5).unwrap()).unwrap();
        let height = PositiveLength::new(Length::from_raw(10).unwrap()).unwrap();
        let frames = vec![
            PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: Rect::new(Length::ZERO, Length::ZERO, half, height),
            },
            PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 1,
                bounds: Rect::new(Length::from_raw(5).unwrap(), Length::ZERO, half, height),
            },
        ];
        let mut plan = page(0, "d");
        plan.frames = frames;
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();

        let next =
            FlowCursor::at(&flow, 1, typaxis_layout::CursorPosition::ParagraphItem(0)).unwrap();
        let first_request = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        assert_eq!(
            permit.run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &first_request,
                PageFrameKind::Header,
                0,
            ),
            Err(FragmentError::UnsupportedFlowDomain)
        );
        permit
            .run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &first_request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();

        let replay = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[1].bounds,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        assert_eq!(
            permit.run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &replay,
                PageFrameKind::Body,
                1,
            ),
            Err(FragmentError::InvalidPageContext)
        );

        let final_fragment = FragmentDraft::new(
            flow.positions()[1].clone(),
            flow.positions().last().unwrap().clone(),
            plan.frames[1].bounds,
            0,
        )
        .unwrap();
        let second_request = FragmentRequest::new(
            &flow,
            &next,
            plan.frames[1].bounds,
            NonNegativeLength::ZERO,
            page_context,
        )
        .unwrap();
        let final_receipt = permit
            .run_fragmenter(
                &OneFragmenter {
                    fragment: final_fragment,
                    terminal: flow.terminal_cursor(),
                    discovered_anchors: vec![DiscoveredAnchor {
                        anchor_id: AnchorId::new("chapter").unwrap(),
                        owner_node: NodeId::new(2),
                        position_in_frame: Point {
                            x: Length::ZERO,
                            y: Length::ZERO,
                        },
                    }],
                },
                &second_request,
                PageFrameKind::Body,
                1,
            )
            .unwrap();
        plan.fragments
            .extend_from_slice(final_receipt.placed_fragments());
        permit.finish_page(&plan).unwrap();
        let receipt = permit.finish(&flow, core::slice::from_ref(&plan)).unwrap();
        assert!(LayoutPass::new(
            receipt,
            input.initial_fingerprint(),
            &flow,
            vec![plan],
            generated,
        )
        .is_ok());
    }

    #[test]
    fn page_frame_geometry_is_rejected_before_page_work_begins() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), default_options(), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&pagination_input))
            .unwrap();
        let (context, cursor, _flow) = blank_page_context(0, &masters);
        let other_package = validated_package_with_uri("other-input.tsf");
        let other_context = other_package.pagination_context();
        let other_generated = generated_store();
        let other_flow =
            FlowTree::empty(&other_package, epoch_for(&other_package, &other_generated)).unwrap();
        let other_cursor = FlowCursor::document_start(&other_flow);
        let other_selection =
            typaxis_layout::ResolvedPageSelection::new(&other_flow, &other_cursor, &other_package)
                .unwrap();
        let other_page = PageContext::select(0, &other_selection, &other_context).unwrap();
        let valid = page(0, "a");
        assert_eq!(
            permit.begin_page(&other_page, &other_cursor, &valid.frames),
            Err(PaginationError::InvalidWorkPermit)
        );
        let size = PositiveLength::new(Length::from_raw(10).unwrap()).unwrap();
        let outside = [PageFramePlan {
            kind: PageFrameKind::Body,
            column_index: 0,
            bounds: Rect::new(Length::from_raw(1).unwrap(), Length::ZERO, size, size),
        }];
        assert_eq!(
            permit.begin_page(&context, &cursor, &outside),
            Err(PaginationError::FrameOutsideMaster)
        );

        assert!(permit.begin_page(&context, &cursor, &valid.frames).is_ok());
        assert!(permit.finish_page(&valid).is_ok());
    }

    #[test]
    fn fingerprint_record_canonicalizes_insertion_order() {
        let mut forward = page(0, "a");
        let mut reverse = page(0, "a");
        let bounds = forward.frames[0].bounds;
        let decision = |ordinal| FloatDecision {
            owner: NodeId::new(0),
            owner_local_ordinal: ordinal,
            frame_kind: PageFrameKind::Body,
            column_index: 0,
            bounds,
        };
        forward.float_decisions = vec![decision(1), decision(0)];
        reverse.float_decisions = vec![decision(0), decision(1)];
        let forward_package = validated_package();
        let forward_store = generated_store();
        let forward_flow = FlowTree::empty(
            &forward_package,
            epoch_for(&forward_package, &forward_store),
        )
        .unwrap();
        let reverse_package = validated_package();
        let reverse_store = generated_store();
        let reverse_flow = FlowTree::empty(
            &reverse_package,
            epoch_for(&reverse_package, &reverse_store),
        )
        .unwrap();
        assert_eq!(
            PaginationFingerprintRecord::new(&forward_flow, vec![forward], forward_store, vec![],)
                .unwrap(),
            PaginationFingerprintRecord::new(&reverse_flow, vec![reverse], reverse_store, vec![],)
                .unwrap()
        );
    }

    #[test]
    fn fingerprint_is_derived_and_length_domain_is_bounded() {
        let first_package = validated_package();
        let first_store = generated_store();
        let first_flow =
            FlowTree::empty(&first_package, epoch_for(&first_package, &first_store)).unwrap();
        let first =
            PaginationFingerprintRecord::new(&first_flow, vec![page(0, "a")], first_store, vec![])
                .unwrap();
        let second_package = validated_package();
        let second_store = generated_store();
        let second_flow =
            FlowTree::empty(&second_package, epoch_for(&second_package, &second_store)).unwrap();
        let second = PaginationFingerprintRecord::new(
            &second_flow,
            vec![page(0, "b")],
            second_store,
            vec![],
        )
        .unwrap();
        assert_ne!(first.fingerprint(), second.fingerprint());

        let mut invalid = page(0, "a");
        let one = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
        assert!(Length::from_raw(i64::MAX).is_none());
        invalid.frames[0].bounds = Rect::new(
            Length::from_raw(JSON_SAFE_INTEGER_MAX).unwrap(),
            Length::ZERO,
            one,
            one,
        );
        let invalid_package = validated_package();
        let invalid_store = generated_store();
        let invalid_flow = FlowTree::empty(
            &invalid_package,
            epoch_for(&invalid_package, &invalid_store),
        )
        .unwrap();
        assert!(PaginationFingerprintRecord::new(
            &invalid_flow,
            vec![invalid],
            invalid_store,
            vec![],
        )
        .is_ok());
    }

    #[test]
    fn result_rejects_passes_after_an_observed_terminal_state() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(3, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let stable_input = transitioned_input(&first);
        let stable = pass(&mut budget, stable_input, score(0, 0), &["a"]);
        let after_input = transitioned_input(&stable);
        let after = pass(&mut budget, after_input, score(0, 0), &["b"]);
        assert_eq!(
            PaginationResult::new(
                vec![first, stable, after],
                ConvergenceStatus::MaxPassFallback,
                &pagination_input,
                budget.finish(),
            ),
            Err(PaginationError::PassesContinueAfterTermination)
        );
    }
}
