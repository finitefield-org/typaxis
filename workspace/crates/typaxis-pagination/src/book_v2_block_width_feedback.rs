//! Widths of fixed-size block objects' parent frames on source-closed pages.
use super::*;

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn physical_block_parent_width(
        &mut self,
        page: &BookV2BodyMixedPlacedPage<'_, 'b, 'f, 's, 'p, 'a>,
        placed: &ProductionBodyFootnotePlacedFragment,
    ) -> Result<PositiveLength, ProductionBodyPaginationError> {
        let owner = placed.fragment().owner();
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        for _ in 0..frames.measurement_region_lookup_work() {
            self.content.step(owner)?;
        }
        let parent = frames
            .measurement_region(owner)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let (measured, actual) = if placed.definition_index().is_some() {
            (
                frames
                    .footnote_region()
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                page.selection()
                    .declared_footnote_region()
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
            )
        } else {
            (frames.body(), page.selection().body_bounds())
        };
        let delta = actual
            .width()
            .get()
            .checked_sub(measured.width().get())
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        if self.fragment_root_table(placed)?.is_some() {
            for _ in 0..frames.inherited_block_lookup_work() {
                self.content.step(owner)?;
            }
            return frames
                .inherited_block_region(owner)
                .map(|f| f.width())
                .ok_or_else(|| error(owner, E::ReceiptMismatch));
        }
        parent
            .width()
            .get()
            .checked_add(delta)
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::WidthMismatch))
    }

    pub(super) fn collect_block_width_feedback(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<(Vec<(NodeId, PositiveLength)>, bool), ProductionBodyPaginationError> {
        closed.require_single_measurement("table_header_variant_width_feedback")?;
        self.collect_block_width_feedback_inner(closed, false)
    }
    pub(super) fn collect_block_width_feedback_inner(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
        skip_tables: bool,
    ) -> Result<(Vec<(NodeId, PositiveLength)>, bool), ProductionBodyPaginationError> {
        if !skip_tables {
            closed.require_single_measurement("table_header_variant_width_feedback")?;
        }
        use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
        let root = NodeId::new(0);
        let lines = self.content.flow.lines();
        let prepared = lines.prepared();
        let count = prepared
            .figures()
            .len()
            .checked_add(
                prepared
                    .native_math()
                    .map_or(0, |m| m.display_blocks().len()),
            )
            .and_then(|n| n.checked_add(self.content.flow.blocks().map_or(0, |b| b.blocks().len())))
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        if count == 0 {
            return Ok((Vec::new(), true));
        }
        self.content.charge.take(
            count
                .checked_mul(2)
                .and_then(|n| n.checked_add(2))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut widths = Vec::new();
        let mut visits = Vec::new();
        widths
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        visits
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let frames = lines
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        for event in prepared.source_flow().events() {
            self.content.step(root)?;
            let Event::Begin {
                owner,
                kind:
                    Region::Figure
                    | Region::VectorFigure
                    | Region::DisplayMath
                    | Region::MathVectorBlock,
            } = *event
            else {
                continue;
            };
            if widths.len() >= count {
                return Err(error(owner, E::ReceiptMismatch));
            }
            for _ in 0..frames.measurement_region_lookup_work() {
                self.content.step(owner)?;
            }
            let frame = frames
                .measurement_region(owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            widths.push((owner, frame.width()));
            visits.push(0u8);
        }
        if widths.len() != count {
            return Err(error(root, E::ReceiptMismatch));
        }
        let mut matches = true;
        for page in closed.geometry().pages() {
            self.content.step(root)?;
            for (fragment_index, (placed, _, repeated)) in page.fragments_with_roles().enumerate() {
                let fragment = placed.fragment();
                let owner = fragment.owner();
                self.content.step(owner)?;
                if matches!(
                    fragment.source(),
                    ProductionBodyFragmentSource::ParagraphLine { .. }
                ) {
                    continue;
                }
                if skip_tables && !page.header_variants().is_empty() {
                    self.source_lookup_work(page.header_variants().len(), owner)?;
                    if page.header_variant(fragment_index).is_some() {
                        continue; // Variant table frames are checked by the occurrence report.
                    }
                }
                if skip_tables && self.fragment_root_table(placed)?.is_some() {
                    continue; // The occurrence projection supplies both dimensions.
                }
                let width = self.physical_block_parent_width(page, placed)?;
                for _ in 0..count {
                    self.content.step(owner)?;
                }
                let index = widths
                    .iter()
                    .position(|(id, _)| *id == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let seen = &mut visits[index];
                if *seen != 0 && widths[index].1 != width {
                    return Err(error(owner, E::WidthMismatch));
                }
                if !repeated && *seen & 1 != 0 {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                *seen |= if repeated { 2 } else { 1 };
                widths[index].1 = width;
                matches &= frames.region(owner).map(|f| f.width()) == Some(width);
            }
        }
        for ((owner, _), seen) in widths.iter().zip(visits) {
            self.content.step(*owner)?;
            if seen != 0 && seen & 1 == 0 {
                return Err(error(*owner, E::ReceiptMismatch));
            }
        }
        Ok((widths, matches))
    }
}
