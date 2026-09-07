//! Simultaneous fit of one actual body boundary and its demanded footnotes.
use super::*;

pub struct ProductionBodyFootnoteCandidate<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    state_id: u64,
    body: std::ops::Range<usize>,
    body_height: Length,
    demanded: ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    footnotes: Option<ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
    footnote_bounds: Option<Rect>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyFootnoteCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn body_range(&self) -> std::ops::Range<usize> {
        self.body.clone()
    }
    pub const fn body_height(&self) -> Length {
        self.body_height
    }
    /// Includes the existing separator band, bottom-aligned in the declared region.
    pub const fn footnote_bounds(&self) -> Option<Rect> {
        self.footnote_bounds
    }
    pub fn footnotes(&self) -> Option<&ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a>> {
        self.footnotes.as_ref()
    }
    pub fn next_state(&self) -> &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        self.footnotes
            .as_ref()
            .map_or(&self.demanded, |f| f.next_state())
    }
    pub fn verify(
        &self,
        state: &ProductionFootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.owner_id != state.owner_id
            || self.state_id != state.state_id
            || !std::ptr::eq(self.demanded.flow, state.flow)
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Validates a local body cut and simultaneous measured fit. It does not
    /// certify prior/next body cursor continuity, assign a page number, rank
    /// alternative body candidates or issue a final page/paint receipt.
    pub fn evaluate_body_candidate(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        range: std::ops::Range<usize>,
    ) -> Result<
        Option<ProductionBodyFootnoteCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(state)?;
        let root = NodeId::new(0);
        let flow = self.content.flow;
        let items = flow.body_items();
        if range.start > range.end || range.end > items.len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.content.charge.take(1, root)?;
        if (!range.is_empty() && range.start > 0 && items[range.start - 1].keep)
            || (range.end > range.start && range.end < items.len() && items[range.end - 1].keep)
        {
            return Ok(None);
        }
        let mut height = Length::ZERO;
        for index in range.clone() {
            let item = &items[index];
            self.step(item.owner)?;
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
        let body = flow.blocks.page_geometry().body();
        if height > body.height().get() {
            return Ok(None);
        }
        self.query_work()?;
        for reference in flow.references_in_items(None, range.clone()) {
            self.step(reference.source().owner())?;
            if reference.first_item_index() < range.start
                || reference.last_item_index() >= range.end
            {
                return Ok(None);
            }
        }
        let demanded = self.require_body(state, range.clone())?;
        if demanded.pending.is_empty() {
            return Ok(Some(ProductionBodyFootnoteCandidate {
                owner_id: self.owner_id,
                state_id: state.state_id,
                body: range,
                body_height: height,
                demanded,
                footnotes: None,
                footnote_bounds: None,
            }));
        }
        let maximum = flow
            .footnote_region()
            .ok_or_else(|| error(root, E::PendingRegion("footnote_frame")))?;
        let bottom = add(maximum.y(), maximum.height().get(), root)?;
        let body_bottom = add(body.y(), height, root)?;
        let horizontal_overlap = body.x() < add(maximum.x(), maximum.width().get(), root)?
            && maximum.x() < add(body.x(), body.width().get(), root)?;
        let reservation_capacity =
            if height > Length::ZERO && horizontal_overlap && bottom > body.y() {
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
        let Some(footnotes) = self.select_required_region(&demanded, capacity)? else {
            return Ok(None);
        };
        for selected in footnotes.fragments() {
            self.query_work()?;
            for reference in selected.fragment().references() {
                self.step(reference.source().owner())?;
                if matches!(footnotes.next_state().definitions[reference.source().definition_index()], Demand::Pending { cursor, .. } if cursor.next_item == 0)
                {
                    return Ok(None);
                }
            }
        }
        let footnote_bounds = if footnotes.used_height() > Length::ZERO {
            let reservation = add(separator, footnotes.used_height(), root)?;
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
        Ok(Some(ProductionBodyFootnoteCandidate {
            owner_id: self.owner_id,
            state_id: state.state_id,
            body: range,
            body_height: height,
            demanded,
            footnotes: Some(footnotes),
            footnote_bounds,
        }))
    }
}
