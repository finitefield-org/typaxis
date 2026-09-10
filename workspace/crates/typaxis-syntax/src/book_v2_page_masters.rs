//! Source-bound page-master choice. A choice grants no layout or paint authority.
use super::StyledBookV2Body;
use typaxis_document_package::{WireAdvancedPageMaster, WirePageMaster, WirePageParity};
use typaxis_style::{page_master_rule_priority, PageParity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookV2PageMasterError {
    PageLimit,
    WorkLimit,
    Identity,
    Geometry,
    HorizontalReflow,
    MissingFootnoteRegion,
    UnsupportedAdvanced,
    RecordLimit,
    SpoolLimit,
    Allocation,
}
impl std::fmt::Display for BookV2PageMasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 page master: {self:?}")
    }
}
impl std::error::Error for BookV2PageMasterError {}
#[derive(Clone, Copy)]
pub struct BookV2SelectedPageMaster<'a> {
    body: &'a StyledBookV2Body,
    page: u32,
    base: &'a WirePageMaster,
    advanced: &'a WireAdvancedPageMaster,
}
impl<'a> BookV2SelectedPageMaster<'a> {
    pub fn source(self) -> &'a StyledBookV2Body {
        self.body
    }
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn master(self) -> &'a WirePageMaster {
        self.base
    }
    pub fn advanced(self) -> &'a WireAdvancedPageMaster {
        self.advanced
    }
}
/// Scans source rules once without allocation. Every visited rule/master is
/// charged before examination; failed choices do not refund the caller's work.
pub fn select_book_v2_page_master<'a>(
    body: &'a StyledBookV2Body,
    page: u32,
    named_page: Option<&str>,
    work: &mut u64,
    maximum: u64,
) -> Result<BookV2SelectedPageMaster<'a>, BookV2PageMasterError> {
    use BookV2PageMasterError as E;
    if page >= body.body().limits().get().max_pages || page.checked_add(1).is_none() {
        return Err(E::PageLimit);
    }
    let mut step = || {
        *work = work
            .checked_add(1)
            .filter(|n| *n <= maximum)
            .ok_or(E::WorkLimit)?;
        Ok::<_, E>(())
    };
    let wire = body.body().wire();
    let masters = wire.page_masters();
    let mut id = masters.default_master_id.as_str();
    let mut winner = None;
    for rule in &masters.selection_rules {
        step()?;
        let parity = match rule.parity {
            WirePageParity::Any => PageParity::Any,
            WirePageParity::Odd => PageParity::Odd,
            WirePageParity::Even => PageParity::Even,
        };
        let priority = page_master_rule_priority(
            page,
            named_page,
            parity,
            rule.first,
            rule.named_page.as_deref(),
            rule.source_order,
        )
        .map_err(|_| E::PageLimit)?;
        if let Some(priority) = priority {
            if winner.is_none_or(|previous| priority > previous) {
                winner = Some(priority);
                id = &rule.master_id;
            }
        }
    }
    // The admitted carrier already requires sorted, unique master identities.
    let mut find = |length: usize, key: &dyn Fn(usize) -> &'a str| -> Result<usize, E> {
        let (mut low, mut high) = (0, length);
        while low < high {
            step()?;
            let middle = low + (high - low) / 2;
            match key(middle).cmp(id) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Ok(middle),
            }
        }
        Err(E::Identity)
    };
    let base = find(masters.masters.len(), &|i| {
        masters.masters[i].master_id.as_str()
    })?;
    let advanced = wire.advanced_page_masters();
    let extra = find(advanced.masters.len(), &|i| {
        advanced.masters[i].master_id.as_str()
    })?;
    Ok(BookV2SelectedPageMaster {
        body,
        page,
        base: &masters.masters[base],
        advanced: &advanced.masters[extra],
    })
}

/// Source-bound measurement envelope and physical page classes for the default
/// and authored named scopes. Maxima measure content; they are not page geometry.
pub struct BookV2PageFramePlan<'a> {
    source: &'a StyledBookV2Body,
    pages: [Option<BookV2PageFrames>; 3],
    body: typaxis_core::Rect,
    footnote: Option<typaxis_core::Rect>,
    named: Vec<NamedFrames>,
    requests: Vec<(typaxis_core::NodeId, usize)>,
    width_reflow: bool,
    records: u64,
    spool: u64,
}
struct NamedFrames {
    name: String,
    pages: [Option<BookV2PageFrames>; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookV2PageFrames {
    body: typaxis_core::Rect,
    footnote: Option<typaxis_core::Rect>,
}
impl BookV2PageFrames {
    pub fn body(self) -> typaxis_core::Rect {
        self.body
    }
    pub fn footnote(self) -> Option<typaxis_core::Rect> {
        self.footnote
    }
}
impl<'a> BookV2PageFramePlan<'a> {
    pub fn requires_width_reflow(&self) -> bool {
        self.width_reflow
    }
    pub fn source(&self) -> &'a StyledBookV2Body {
        self.source
    }
    pub fn measurement_body(&self) -> typaxis_core::Rect {
        self.body
    }
    pub fn measurement_footnote(&self) -> Option<typaxis_core::Rect> {
        self.footnote
    }
    pub fn minimum_body_height(&self) -> typaxis_core::Length {
        self.all_frames()
            .map(|p| p.body.height().get())
            .min()
            .expect("first page")
    }
    pub fn minimum_footnote_height(&self) -> Option<typaxis_core::Length> {
        self.all_frames()
            .filter_map(|p| p.footnote.map(|r| r.height().get()))
            .min()
    }
    pub fn page(&self, page: u32) -> Result<BookV2PageFrames, BookV2PageMasterError> {
        if page >= self.source.body().limits().get().max_pages || page == u32::MAX {
            return Err(BookV2PageMasterError::PageLimit);
        }
        let index = if page == 0 {
            0
        } else if page % 2 == 1 {
            1
        } else {
            2
        };
        self.pages[index].ok_or(BookV2PageMasterError::PageLimit)
    }
    fn all_frames(&self) -> impl Iterator<Item = &BookV2PageFrames> {
        self.pages
            .iter()
            .flatten()
            .chain(self.named.iter().flat_map(|n| n.pages.iter().flatten()))
    }
    pub fn has_source_names(&self) -> bool {
        !self.requests.is_empty()
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool
    }
    pub fn name(&self, index: usize) -> Option<&str> {
        self.named.get(index).map(|n| n.name.as_str())
    }
    pub fn source_name_index(&self, owner: typaxis_core::NodeId) -> Option<usize> {
        self.requests
            .binary_search_by_key(&owner, |r| r.0)
            .ok()
            .map(|i| self.requests[i].1)
    }
    pub fn named_page(
        &self,
        page: u32,
        name: Option<usize>,
    ) -> Result<BookV2PageFrames, BookV2PageMasterError> {
        let default = self.page(page)?;
        let Some(name) = name else {
            return Ok(default);
        };
        let index = if page == 0 {
            0
        } else if page % 2 == 1 {
            1
        } else {
            2
        };
        self.named
            .get(name)
            .ok_or(BookV2PageMasterError::Identity)?
            .pages[index]
            .ok_or(BookV2PageMasterError::PageLimit)
    }
}
/// Only first/parity rules are reachable without a named source transition.
/// Rule scans and source lookups are charged once; later physical selections
/// are bounded constant-time lookups in this immutable source-bound plan.
pub fn prepare_book_v2_page_frame_plan<'a>(
    source: &'a StyledBookV2Body,
    work: &mut u64,
    maximum: u64,
) -> Result<BookV2PageFramePlan<'a>, BookV2PageMasterError> {
    prepare_frames_for_name(source, None, work, maximum, false)
}
fn prepare_frames_for_name<'a>(
    source: &'a StyledBookV2Body,
    name: Option<&str>,
    work: &mut u64,
    maximum: u64,
    allow_widths: bool,
) -> Result<BookV2PageFramePlan<'a>, BookV2PageMasterError> {
    use typaxis_core::{Length, PositiveLength, Rect};
    use BookV2PageMasterError as E;
    let has_notes = !source.body().wire().document().footnotes.is_empty();
    let mut pages = [None; 3];
    for page in 0..source.body().limits().get().max_pages.min(3) {
        let selected = select_book_v2_page_master(source, page, name, work, maximum)?;
        let advanced = selected.advanced();
        if advanced.header_content.is_some()
            || advanced.footer_content.is_some()
            || advanced.column_layout.is_some()
        {
            return Err(E::UnsupportedAdvanced);
        }
        let master = selected.master();
        let rectangle = |x, y, width, height| -> Result<Rect, E> {
            let len = |n| Length::from_raw(n).ok_or(E::Geometry);
            let (x, y, w, h) = (len(x)?, len(y)?, len(width)?, len(height)?);
            // Preserve the existing source-frame contract: page containment is
            // required by note-region preparation. Note-free authored frames
            // have historically permitted content beyond the media rectangle.
            if has_notes
                && (x < Length::ZERO
                    || y < Length::ZERO
                    || x.checked_add(w)
                        .is_none_or(|n| n > len(master.width).unwrap_or(Length::ZERO))
                    || y.checked_add(h)
                        .is_none_or(|n| n > len(master.height).unwrap_or(Length::ZERO)))
            {
                return Err(E::Geometry);
            }
            Ok(Rect::new(
                x,
                y,
                PositiveLength::new(w).ok_or(E::Geometry)?,
                PositiveLength::new(h).ok_or(E::Geometry)?,
            ))
        };
        let b = &master.body;
        let body = rectangle(b.x, b.y, b.width, b.height)?;
        let footnote = if has_notes {
            let n = master.footnote.as_ref().ok_or(E::MissingFootnoteRegion)?;
            Some(rectangle(n.x, n.y, n.width, n.height)?)
        } else {
            None
        };
        pages[page as usize] = Some(BookV2PageFrames { body, footnote });
    }
    let first = pages[0].ok_or(E::PageLimit)?;
    let mut body = first.body;
    let mut footnote = first.footnote;
    let expand = |envelope: Rect, actual: Rect| -> Result<Rect, E> {
        if !allow_widths && envelope.width() != actual.width() {
            return Err(E::HorizontalReflow);
        }
        Ok(Rect::new(
            envelope.x(),
            envelope.y(),
            if envelope.width().get() < actual.width().get() {
                actual.width()
            } else {
                envelope.width()
            },
            if actual.height().get() > envelope.height().get() {
                actual.height()
            } else {
                envelope.height()
            },
        ))
    };
    let mut width_reflow = false;
    for actual in pages.iter().flatten().skip(1) {
        width_reflow |= body.width() != actual.body.width()
            || matches!((footnote, actual.footnote), (Some(a),Some(b)) if a.width() != b.width());
        body = expand(body, actual.body)?;
        if let (Some(a), Some(b)) = (footnote, actual.footnote) {
            footnote = Some(expand(a, b)?);
        }
    }
    Ok(BookV2PageFramePlan {
        source,
        pages,
        body,
        footnote,
        named: Vec::new(),
        requests: Vec::new(),
        width_reflow,
        records: 0,
        spool: 0,
    })
}

/// Resolve the used page name of each body region using source nesting. An auto
/// child remains in its enclosing named region; a nested explicit name overrides
/// that region until its End event. Footnote definitions do not select body pages.
pub fn prepare_book_v2_page_frame_plan_for_flow<'a>(
    flow: &super::PreparedBookV2TextFlow<'a>,
    work: &mut u64,
    maximum: u64,
) -> Result<BookV2PageFramePlan<'a>, BookV2PageMasterError> {
    prepare_book_v2_page_frame_plan_for_flow_with_prior(flow, work, maximum, 0, 0)
}
pub fn prepare_book_v2_page_frame_plan_for_flow_with_prior<'a>(
    flow: &super::PreparedBookV2TextFlow<'a>,
    work: &mut u64,
    maximum: u64,
    prior_records: u64,
    prior_spool: u64,
) -> Result<BookV2PageFramePlan<'a>, BookV2PageMasterError> {
    prepare_flow_frames(flow, work, maximum, prior_records, prior_spool, false)
}
/// Retain maximum-width measurement envelopes for source-aware page feedback.
/// Tables must remeasure columns/cells and verify actual continuation widths.
pub fn prepare_book_v2_page_frame_plan_for_reflow<'a>(
    flow: &super::PreparedBookV2TextFlow<'a>,
    work: &mut u64,
    maximum: u64,
    prior_records: u64,
    prior_spool: u64,
) -> Result<BookV2PageFramePlan<'a>, BookV2PageMasterError> {
    prepare_flow_frames(flow, work, maximum, prior_records, prior_spool, true)
}
fn prepare_flow_frames<'a>(
    flow: &super::PreparedBookV2TextFlow<'a>,
    work: &mut u64,
    maximum: u64,
    prior_records: u64,
    prior_spool: u64,
    allow_widths: bool,
) -> Result<BookV2PageFramePlan<'a>, BookV2PageMasterError> {
    use crate::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
    use typaxis_core::{NodeId, Rect};
    use BookV2PageMasterError as E;
    let source = flow.body();
    let mut plan = prepare_frames_for_name(source, None, work, maximum, allow_widths)?;
    let caps = source.body().limits().get();
    if prior_records > caps.max_fragments {
        return Err(E::RecordLimit);
    }
    if prior_spool > caps.max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    plan.records = prior_records;
    plan.spool = prior_spool;
    let step = |work: &mut u64, n: u64| -> Result<(), E> {
        *work = work
            .checked_add(n)
            .filter(|n| *n <= maximum)
            .ok_or(E::WorkLimit)?;
        Ok(())
    };
    let record = |plan: &mut BookV2PageFramePlan<'_>, n: u64| -> Result<(), E> {
        plan.records = plan
            .records
            .checked_add(n)
            .filter(|n| *n <= caps.max_fragments)
            .ok_or(E::RecordLimit)?;
        Ok(())
    };
    // The exact source flow also retains explicit break-name strings during
    // each line pass. Include that live storage and its sort in command bounds.
    let breaks = flow.named_page_breaks();
    record(&mut plan, breaks.len() as u64)?;
    for (_, name) in breaks {
        plan.spool = plan
            .spool
            .checked_add(name.as_str().len() as u64)
            .filter(|n| *n <= caps.max_spool_bytes)
            .ok_or(E::SpoolLimit)?;
    }
    step(
        work,
        breaks.len() as u64 * (u64::from(breaks.len().checked_ilog2().unwrap_or(0)) + 1),
    )?;
    let mut stack: Vec<(NodeId, Option<usize>)> = Vec::new();
    let mut names = std::collections::BTreeMap::<&str, usize>::new();
    let (mut paragraph, mut list, mut description, mut figure, mut table) = (0, 0, 0, 0, 0);
    for event in flow.events() {
        step(work, 1)?;
        match *event {
            Event::Begin {
                kind: Region::Footnote,
                ..
            } => {
                if !stack.is_empty() {
                    return Err(E::Identity);
                }
                break;
            }
            Event::Begin { owner, kind } => {
                let local = match kind {
                    Region::Paragraph | Region::Heading | Region::DescriptionTerm => flow
                        .paragraphs()
                        .get(paragraph)
                        .filter(|p| p.owner() == owner)
                        .ok_or(E::Identity)?
                        .page_name(),
                    Region::List => {
                        let v = flow
                            .lists()
                            .get(list)
                            .filter(|p| p.owner() == owner)
                            .ok_or(E::Identity)?;
                        list += 1;
                        v.page_name()
                    }
                    Region::DescriptionList => {
                        let v = flow
                            .description_lists()
                            .get(description)
                            .filter(|p| p.owner() == owner)
                            .ok_or(E::Identity)?;
                        description += 1;
                        v.page_name()
                    }
                    Region::Figure => {
                        let v = flow
                            .figures()
                            .get(figure)
                            .filter(|p| p.owner() == owner)
                            .ok_or(E::Identity)?;
                        figure += 1;
                        v.page_name()
                    }
                    Region::Table => {
                        let v = flow
                            .tables()
                            .get(table)
                            .filter(|p| p.owner() == owner)
                            .ok_or(E::Identity)?;
                        table += 1;
                        v.page_name()
                    }
                    Region::PageBreak => {
                        step(
                            work,
                            u64::from(breaks.len().checked_ilog2().unwrap_or(0)) + 1,
                        )?;
                        flow.page_break_name(owner)
                    }
                    Region::SemanticContainer => source
                        .container_style(owner)
                        .ok_or(E::Identity)?
                        .page_name(),
                    Region::DisplayMath => source.math_style(owner).ok_or(E::Identity)?.page_name(),
                    Region::VectorFigure | Region::MathVectorBlock => {
                        source.vector_style(owner).ok_or(E::Identity)?.page_name()
                    }
                    _ => None,
                };
                let selected = if let Some(name) = local {
                    let name = name.as_str();
                    step(
                        work,
                        u64::from(names.len().checked_ilog2().unwrap_or(0)) + 1,
                    )?;
                    if let Some(index) = names.get(name) {
                        Some(*index)
                    } else {
                        record(&mut plan, 5)?;
                        plan.spool = plan
                            .spool
                            .checked_add(name.len() as u64)
                            .filter(|n| *n <= caps.max_spool_bytes)
                            .ok_or(E::SpoolLimit)?;
                        let frames = prepare_frames_for_name(
                            source,
                            Some(name),
                            work,
                            maximum,
                            allow_widths,
                        )?;
                        let mut owned = String::new();
                        owned
                            .try_reserve_exact(name.len())
                            .map_err(|_| E::Allocation)?;
                        owned.push_str(name);
                        plan.named.try_reserve(1).map_err(|_| E::Allocation)?;
                        let index = plan.named.len();
                        plan.named.push(NamedFrames {
                            name: owned,
                            pages: frames.pages,
                        });
                        names.insert(name, index);
                        Some(index)
                    }
                } else {
                    stack.last().and_then(|s| s.1)
                };
                record(&mut plan, 1)?;
                stack.try_reserve(1).map_err(|_| E::Allocation)?;
                stack.push((owner, selected));
                if let Some(index) = selected {
                    record(&mut plan, 1)?;
                    plan.requests.try_reserve(1).map_err(|_| E::Allocation)?;
                    plan.requests.push((owner, index));
                }
            }
            Event::Paragraph { index } => {
                if index as usize != paragraph {
                    return Err(E::Identity);
                }
                paragraph += 1;
            }
            Event::End { owner, .. } => {
                if stack.pop().map(|s| s.0) != Some(owner) {
                    return Err(E::Identity);
                }
            }
        }
    }
    if !stack.is_empty() {
        return Err(E::Identity);
    }
    if !plan.requests.is_empty() {
        step(
            work,
            plan.requests.len() as u64 * (u64::from(plan.requests.len().ilog2()) + 1),
        )?;
        plan.requests.sort_unstable_by_key(|r| r.0);
        if plan.requests.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(E::Identity);
        }
    }
    let expand = |a: Rect, b: Rect| -> Result<Rect, E> {
        if !allow_widths && a.width() != b.width() {
            return Err(E::HorizontalReflow);
        }
        Ok(Rect::new(
            a.x(),
            a.y(),
            if a.width().get() < b.width().get() {
                b.width()
            } else {
                a.width()
            },
            if a.height().get() < b.height().get() {
                b.height()
            } else {
                a.height()
            },
        ))
    };
    for named in &plan.named {
        for actual in named.pages.iter().flatten() {
            step(work, 1)?;
            plan.width_reflow |= plan.body.width() != actual.body.width()
                || matches!((plan.footnote,actual.footnote),(Some(a),Some(b)) if a.width()!=b.width());
            plan.body = expand(plan.body, actual.body)?;
            if let (Some(a), Some(b)) = (plan.footnote, actual.footnote) {
                plan.footnote = Some(expand(a, b)?);
            }
        }
    }
    if plan.width_reflow {
        // Preserve the source-plan scan charge while table compatibility is now
        // verified from actual selected continuations and remeasured columns.
        for _ in flow.events() {
            step(work, 1)?;
        }
    }
    plan.records -= prior_records;
    plan.spool -= prior_spool;
    Ok(plan)
}
