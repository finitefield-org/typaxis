//! Shared body-cut validation and simultaneous footnote reservation.
use super::*;
use region_kernel::RegionSearch;

pub(super) trait FitSearch: RegionSearch {
    type Region;
    fn body(&self) -> Rect;
    fn footnote_region(&self) -> Option<Rect>;
    fn pending(&self, state: &Self::State) -> bool;
    fn required(
        &mut self,
        state: &Self::State,
        capacity: Length,
    ) -> Result<Option<Self::Region>, ProductionBodyPaginationError>;
    /// Successor dependency search; frozen wrappers retain their prior refusal.
    fn resolve_unstarted(
        &mut self,
        _state: &Self::State,
        _capacity: Length,
    ) -> Result<Option<Self::Region>, ProductionBodyPaginationError> {
        Ok(None)
    }
    fn fragments(region: &Self::Region) -> &[Self::Fragment];
    fn references(fragment: &Self::Fragment) -> impl Iterator<Item = (NodeId, usize)>;
    fn unstarted(region: &Self::Region, definition: usize) -> bool;
    fn height(region: &Self::Region) -> Length;
    fn query_work(&mut self) -> Result<(), ProductionBodyPaginationError>;
}
pub(super) struct Fit<S: FitSearch> {
    pub demanded: S::State,
    pub footnotes: Option<S::Region>,
    pub footnote_bounds: Option<Rect>,
}
pub(super) fn fit<S: FitSearch>(
    search: &mut S,
    height: Length,
    demanded: S::State,
) -> Result<Option<Fit<S>>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let body = search.body();
    if !search.pending(&demanded) {
        return Ok(Some(Fit {
            demanded,
            footnotes: None,
            footnote_bounds: None,
        }));
    }
    let maximum = search
        .footnote_region()
        .ok_or_else(|| error(root, E::PendingRegion("footnote_frame")))?;
    let bottom = add(maximum.y(), maximum.height().get(), root)?;
    let body_bottom = add(body.y(), height, root)?;
    let horizontal_overlap = body.x() < add(maximum.x(), maximum.width().get(), root)?
        && maximum.x() < add(body.x(), body.width().get(), root)?;
    let reservation_capacity = if height > Length::ZERO && horizontal_overlap && bottom > body.y() {
        bottom
            .checked_sub(body_bottom)
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?
            .max(Length::ZERO)
            .min(maximum.height().get())
    } else {
        maximum.height().get()
    };
    let separator = Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW)
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
    let capacity = reservation_capacity
        .checked_sub(separator)
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?
        .max(Length::ZERO);
    let Some(mut footnotes) = search.required(&demanded, capacity)? else {
        return Ok(None);
    };
    if !references_started(search, &footnotes)? {
        let Some(closed) = search.resolve_unstarted(&demanded, capacity)? else {
            return Ok(None);
        };
        if !references_started(search, &closed)? {
            return Err(error(root, E::ReceiptMismatch));
        }
        footnotes = closed;
    }
    let footnote_bounds = if S::height(&footnotes) > Length::ZERO {
        let reservation = add(separator, S::height(&footnotes), root)?;
        if reservation > reservation_capacity {
            return Ok(None);
        }
        let y = bottom
            .checked_sub(reservation)
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        Some(Rect::new(
            maximum.x(),
            y,
            maximum.width(),
            PositiveLength::new(reservation).ok_or_else(|| error(root, E::ReceiptMismatch))?,
        ))
    } else {
        None
    };
    Ok(Some(Fit {
        demanded,
        footnotes: Some(footnotes),
        footnote_bounds,
    }))
}

fn references_started<S: FitSearch>(
    search: &mut S,
    region: &S::Region,
) -> Result<bool, ProductionBodyPaginationError> {
    for selected in S::fragments(region) {
        search.query_work()?;
        for (owner, definition) in S::references(selected) {
            search.step(owner)?;
            if S::unstarted(region, definition) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(super) trait BodySearch<'b>: FitSearch {
    fn items(&self) -> &'b [ProductionBodyFlowItem];
    fn allow_range(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<(), ProductionBodyPaginationError>;
    fn keep_before(&mut self, start: usize) -> Result<bool, ProductionBodyPaginationError>;
    fn complete_references(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError>;
    fn require_body(
        &mut self,
        state: &Self::State,
        range: std::ops::Range<usize>,
    ) -> Result<Self::State, ProductionBodyPaginationError>;
}
pub(super) fn body_candidate<'b, S: BodySearch<'b>>(
    search: &mut S,
    state: &S::State,
    range: std::ops::Range<usize>,
) -> Result<Option<(Length, Fit<S>)>, ProductionBodyPaginationError> {
    search.verify(state)?;
    let root = NodeId::new(0);
    let items = search.items();
    if range.start > range.end || range.end > items.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    search.allow_range(range.clone())?;
    search.charge(1, root)?;
    if (!range.is_empty() && search.keep_before(range.start)?)
        || (range.end > range.start && range.end < items.len() && items[range.end - 1].keep)
    {
        return Ok(None);
    }
    let mut height = Length::ZERO;
    for index in range.clone() {
        let item = &items[index];
        search.step(item.owner)?;
        if item.source.is_none() {
            return Ok(None);
        }
        let gap = if index == range.start {
            Length::ZERO
        } else {
            add(items[index - 1].after, item.before, item.owner)?
        };
        height = add(add(height, gap, item.owner)?, item.consumed()?, item.owner)?;
    }
    if height > search.body().height().get() {
        return Ok(None);
    }
    if !search.complete_references(range.clone())? {
        return Ok(None);
    }
    let demanded = search.require_body(state, range)?;
    Ok(fit(search, height, demanded)?.map(|fit| (height, fit)))
}
pub(super) fn complete_references(
    references: &[ProductionFootnoteFlowReference<'_>],
    range: std::ops::Range<usize>,
    steps: &mut u64,
    maximum: u64,
) -> Result<bool, ProductionBodyPaginationError> {
    for reference in references {
        visit(steps, maximum, reference.source().owner())?;
        if reference.first_item_index() < range.start || reference.last_item_index() >= range.end {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) trait TableSearch: FitSearch {
    type TableFragment;
    fn verify_table(
        &self,
        fragment: &Self::TableFragment,
    ) -> Result<(), ProductionBodyPaginationError>;
    fn ranges(fragment: &Self::TableFragment) -> impl Iterator<Item = std::ops::Range<usize>>;
    fn table_height(fragment: &Self::TableFragment) -> Length;
    fn fork_table(
        &mut self,
        state: &Self::State,
    ) -> Result<Self::State, ProductionBodyPaginationError>;
    fn require_range(
        &mut self,
        state: &mut Self::State,
        range: std::ops::Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError>;
}
pub(super) fn table_demand<S: TableSearch>(
    search: &mut S,
    state: &S::State,
    fragment: Option<&S::TableFragment>,
) -> Result<Option<Fit<S>>, ProductionBodyPaginationError> {
    search.verify(state)?;
    let root = NodeId::new(0);
    search.charge(1, root)?;
    if let Some(fragment) = fragment {
        search.verify_table(fragment)?;
    }
    let mut demanded = search.fork_table(state)?;
    if let Some(fragment) = fragment {
        for range in S::ranges(fragment) {
            search.step(root)?;
            if !search.require_range(&mut demanded, range)? {
                return Ok(None);
            }
        }
    }
    fit(
        search,
        fragment.map_or(Length::ZERO, S::table_height),
        demanded,
    )
}
