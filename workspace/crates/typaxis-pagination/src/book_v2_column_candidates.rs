//! Joint physical-page candidates: ordered body columns, one demand branch and
//! one full-page note reservation. These are not stable placement/PDF receipts.
use super::*;
use crate::book_v2::{BookV2ColumnTableMeasurements, BookV2TableCursor};
use typaxis_syntax::book_v2::{BookV2ColumnFramePlan, BookV2ColumnPageFrames};
#[path = "book_v2_column_pages.rs"]
mod pages;
pub use pages::*;
#[path = "book_v2_column_placement.rs"]
mod placement_columns;
pub use placement_columns::{BookV2ColumnPlacedBody, BookV2ColumnPlacedHeaderVariant, BookV2ColumnPlacedPage, BookV2ColumnPlacedSequence};
#[path = "book_v2_column_stability.rs"]
mod stable_columns;
pub use stable_columns::BookV2ColumnStablePages;

pub struct BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) inner: BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>,
    pub(super) plan: &'p BookV2ColumnFramePlan<'a>,
}
pub struct BookV2ColumnPageState<'b, 'f, 's, 'p, 'a> {
    pub(super) source: BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
    page: u32,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageState<'b, 'f, 's, 'p, 'a> {
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn next_item(&self) -> usize {
        self.source.next_item()
    }
    pub fn next_table_index(&self) -> usize {
        self.source.next_table_index()
    }
    pub fn table_continuation(&self) -> Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>> {
        self.source.table_continuation()
    }
    pub fn pending_definitions(&self) -> &[usize] {
        self.source.demand.pending_definitions()
    }
    pub fn definition_status(&self, index: usize) -> Option<ProductionFootnoteDemandStatus> {
        self.source.demand.status(index)
    }
    pub fn is_complete(&self) -> bool {
        self.source.is_complete()
    }
}
pub struct BookV2ColumnBodyCandidate<'b, 'f, 's, 'p, 'a> {
    bounds: Rect,
    parts: Vec<BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>>,
    height: Length,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnBodyCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn parts(&self) -> &[BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>] {
        &self.parts
    }
    pub fn used_height(&self) -> Length {
        self.height
    }
}
/// A candidate cannot be converted into the stable single-body page receipt.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnPageCandidate, BookV2BodyMixedPageSelection};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a> {
    frames: BookV2ColumnPageFrames<'a>,
    columns: Vec<BookV2ColumnBodyCandidate<'b, 'f, 's, 'p, 'a>>,
    fit: BookV2BodyFootnoteFit<'b, 'f, 's, 'p, 'a>,
    next: BookV2ColumnPageState<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn frames(&self) -> BookV2ColumnPageFrames<'a> {
        self.frames
    }
    pub fn columns(&self) -> &[BookV2ColumnBodyCandidate<'b, 'f, 's, 'p, 'a>] {
        &self.columns
    }
    pub fn footnotes(&self) -> Option<&BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>> {
        self.fit.footnotes()
    }
    pub fn footnote_bounds(&self) -> Option<Rect> {
        self.fit.footnote_bounds()
    }
    pub fn next_state(&self) -> &BookV2ColumnPageState<'b, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn into_next_state(self) -> BookV2ColumnPageState<'b, 'f, 's, 'p, 'a> {
        self.next
    }
    pub fn verify(
        &self,
        state: &BookV2ColumnPageState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.frames.page_index() != state.page {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        self.fit.verify(&state.source.demand)
    }
}

/// The measurements have no public single-frame escape. All constructor
/// reservations, including failed prefixes, remain in the supplied counters.
pub fn prepare_book_v2_column_page_search_counted<'b, 'f, 's, 'p, 'a>(
    measurements: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let inner = prepare_book_v2_table_body_search_counted(
        &measurements.inner,
        limits,
        maximum_work,
        prior_records,
        observed_records,
        observed_work,
    )?;
    Ok(BookV2ColumnPageSearch {
        inner,
        plan: measurements.column_plan(),
    })
}
/// A column-owned catalog supplies the actual header chosen in every segment.
pub fn prepare_book_v2_column_page_search_with_headers_counted<'b, 'f, 's, 'p, 'a>(
    catalog: &'b crate::book_v2::BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let inner = prepare_book_v2_table_body_search_with_headers_counted(&catalog.inner,
        limits, maximum_work, prior_records, observed_records, observed_work)?;
    Ok(BookV2ColumnPageSearch { inner, plan: catalog.base().column_plan() })
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    /// Original semantic graph; physical repeated headers keep separate owners.
    pub fn source_lines(&self) -> &'s typaxis_layout::book_v2::BookV2InlineLineLayout<'p, 'a> {
        self.inner.content.flow.lines()
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.inner.work_steps()
    }
    pub fn begin(
        &mut self,
    ) -> Result<BookV2ColumnPageState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.inner
            .begin_body_source()
            .map(|source| BookV2ColumnPageState { source, page: 0 })
    }
    pub fn begin_table(
        &mut self,
        index: usize,
    ) -> Result<BookV2TableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.inner.begin_table(index)
    }

    /// Requests cover every authored column in order, including unused suffix
    /// columns. Notes are selected once after all ordinary/table references.
    /// Width feedback, balancing and stable placement belong to the
    /// subsequent stable-page protocol; this candidate cannot authorize paint.
    pub fn evaluate_page(
        &mut self,
        state: &BookV2ColumnPageState<'b, 'f, 's, 'p, 'a>,
        name: Option<usize>,
        requests: &[&[BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a>]],
    ) -> Result<Option<BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        self.evaluate_page_with_blank(state, name, requests, false)
    }
    fn evaluate_page_with_blank(
        &mut self,
        state: &BookV2ColumnPageState<'b, 'f, 's, 'p, 'a>,
        name: Option<usize>,
        requests: &[&[BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a>]],
        allow_blank: bool,
    ) -> Result<Option<BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError> {
        self.inner.verify_state(&state.source.demand)?;
        let root = NodeId::new(0);
        if state.is_complete() && !allow_blank {
            return Ok(None);
        }
        if state.page >= self.inner.maximum_pages {
            return Err(error(root, E::PageLimit));
        }
        let frames = self
            .plan
            .named_page(state.page, name)
            .map_err(|_| error(root, E::ReceiptMismatch))?;
        if requests.len() != usize::from(frames.column_count()) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let saved_frames = self.inner.active_page_frames;
        let saved_name = self.inner.active_page_name;
        let saved_mismatch = self.inner.named_mismatch.take();
        self.inner.active_page_name = name;
        let result = self.evaluate_in_frames(state, frames, requests, allow_blank);
        self.inner.active_page_frames = saved_frames;
        self.inner.active_page_name = saved_name;
        self.inner.named_mismatch = saved_mismatch;
        result
    }

    fn evaluate_in_frames(
        &mut self,
        state: &BookV2ColumnPageState<'b, 'f, 's, 'p, 'a>,
        frames: BookV2ColumnPageFrames<'a>,
        requests: &[&[BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a>]],
        allow_blank: bool,
    ) -> Result<Option<BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        self.inner.content.charge(
            requests
                .len()
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut columns = Vec::new();
        columns
            .try_reserve_exact(requests.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut source = BookV2BodySourceState {
            demand: self.inner.fork(&state.source.demand, 0)?,
            item: state.source.item,
            next_table: state.source.next_table,
            table_end: state.source.table_end,
            continuation: state.source.continuation,
        };
        let mut unused = false;
        let mut closed = false;
        let mut overlapping_height = Length::ZERO;
        for (index, request) in requests.iter().enumerate() {
            self.inner.content.step(root)?;
            if (unused || closed) && !request.is_empty() {
                return Err(error(root, E::ReceiptMismatch));
            }
            unused |= request.is_empty();
            let bounds = frames
                .column(index as u16)
                .map_err(|_| error(root, E::ReceiptMismatch))?;
            self.inner.active_page_frames = Some(ActivePageFrames {
                body: bounds,
                footnote: frames.footnote(),
            });
            self.inner.verify_source_requests(&source, request)?;
            if !request.is_empty()
                && self
                    .inner
                    .current_source_page_name(
                        source.item,
                        source.next_table,
                        source.continuation,
                        source.continuation.map(|_| frames.named_page_index()),
                    )?
                    .is_some_and(|name| name != frames.named_page_index())
            {
                let owner = self
                    .inner
                    .tables
                    .as_ref()
                    .and_then(|tables| {
                        tables
                            .at(source.item, source.next_table)
                            .map(|index| tables.table(index).owner())
                    })
                    .or_else(|| {
                        self.inner
                            .content
                            .flow
                            .body_items()
                            .get(source.item)
                            .map(|item| item.owner())
                    })
                    .unwrap_or(root);
                return Err(error(owner, E::PendingNamedPage));
            }
            let Some(selected) = mixed_kernel::evaluate_source(
                &mut self.inner,
                &source.demand,
                source.item,
                Some(source.next_table),
                source.continuation,
                request,
            )?
            else {
                return Ok(None);
            };
            for part in &selected.parts {
                if let Some(range) = part.items() {
                    self.verify_names(range, frames.named_page_index())?;
                }
                if let Some(table) = part.table() {
                    for range in table.semantic_leaf_ranges() {
                        self.verify_names(range, frames.named_page_index())?;
                    }
                }
            }
            if let Some(note) = frames.footnote() {
                if bounds.x() < add(note.x(), note.width().get(), root)?
                    && note.x() < add(bounds.x(), bounds.width().get(), root)?
                {
                    overlapping_height = overlapping_height.max(selected.height);
                }
            }
            source = BookV2BodySourceState {
                demand: selected.demanded,
                item: selected.end,
                next_table: selected.next_table.expect("column source table ordinal"),
                table_end: source.table_end,
                continuation: selected.continuation,
            };
            closed |= selected.parts.last().and_then(|part| part.table())
                .is_some_and(|table| table.forced_break_owner().is_some())
                || self.outside_source_break(&source, frames.named_page_index())?.is_some();
            columns.push(BookV2ColumnBodyCandidate {
                bounds,
                parts: selected.parts,
                height: selected.height,
            });
        }
        self.inner.active_page_frames = Some(ActivePageFrames {
            body: frames.body(),
            footnote: frames.footnote(),
        });
        let Some(projection) = fit_kernel::fit(&mut self.inner, overlapping_height, source.demand)?
        else {
            if let Some(owner) = self.inner.named_mismatch {
                return Err(error(owner, E::PendingNamedPage));
            }
            return Ok(None);
        };
        let fit = BookV2BodyFootnoteFit::from_projection(&state.source.demand, projection);
        // A notes-only page must actually advance a pending definition. Empty
        // padding columns alone cannot manufacture another physical page.
        let body_progress = source.item != state.source.item
            || source.next_table != state.source.next_table
            || columns.iter().any(|column| !column.parts.is_empty());
        let note_progress = fit
            .footnotes()
            .is_some_and(|notes| !notes.fragments().is_empty());
        if !body_progress && !note_progress && !allow_blank {
            return Ok(None);
        }
        self.inner.content.charge(2, root)?;
        let next = BookV2ColumnPageState {
            page: state
                .page
                .checked_add(1)
                .ok_or_else(|| error(root, E::PageLimit))?,
            source: BookV2BodySourceState {
                demand: self.inner.fork(fit.next_state(), 0)?,
                item: source.item,
                next_table: source.next_table,
                table_end: source.table_end,
                continuation: source.continuation,
            },
        };
        Ok(Some(BookV2ColumnPageCandidate {
            frames,
            columns,
            fit,
            next,
        }))
    }
    fn outside_source_break(
        &mut self,
        source: &BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
        name: Option<usize>,
    ) -> Result<Option<NodeId>, ProductionBodyPaginationError> {
        self.inner.content.step(NodeId::new(0))?;
        Ok((source.continuation.is_none()
            && self.inner.tables.as_ref().unwrap().at(source.item, source.next_table).is_none()
            && self.inner.source_page_name(source.item, source.next_table) == Some(name))
            .then(|| self.inner.content.flow.body_items().get(source.item)
                .filter(|item| item.source.is_none()).map(|item| item.owner))
            .flatten())
    }
    fn verify_names(
        &mut self,
        range: std::ops::Range<usize>,
        name: Option<usize>,
    ) -> Result<(), ProductionBodyPaginationError> {
        for index in range {
            let flow = self.inner.content.flow;
            let item = flow
                .body_items()
                .get(index)
                .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
            self.inner.content.step(item.owner)?;
            if flow.body_page_name_index(index) != name {
                return Err(error(item.owner, E::PendingNamedPage));
            }
        }
        Ok(())
    }
}
