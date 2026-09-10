//! Sparse, exact geometry owners for repeated-header paint on mixed pages.
use super::*;
use crate::book_v2::{BookV2TableHeaderVariant, BookV2TableMeasurements};

/// A placed fragment's indexes belong to this header's measurement, not the base
/// semantic stream. The header association retains the original source identity.
pub struct BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a> {
    pub(super) fragment: usize,
    pub(super) item: usize,
    pub(super) header: &'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a> {
    pub fn fragment_index(&self) -> usize {
        self.fragment
    }
    pub fn global_item_index(&self) -> usize {
        self.item
    }
    pub fn header(&self) -> &'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a> {
        self.header
    }
    pub fn measurements(&self) -> &'b BookV2TableMeasurements<'f, 's, 'p, 'a> {
        self.header.variant()
    }
}
impl PartialEq for BookV2BodyPlacedHeaderVariant<'_, '_, '_, '_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.fragment == other.fragment
            && self.item == other.item
            && std::ptr::eq(self.header, other.header)
    }
}
impl Eq for BookV2BodyPlacedHeaderVariant<'_, '_, '_, '_, '_> {}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn content_placement_for(
        &mut self,
        flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    ) -> placement::ContentPlacement<'_, '_, 'p, 'a> {
        placement::ContentPlacement {
            lines: BodyLines::BookV2(flow.lines()),
            blocks: BodyBlocks::BookV2(flow.blocks().map_or(&[], |b| b.blocks())),
            collected: &flow.collected,
            definition_markers: &flow.definition_markers,
            body: flow.lines().frames().expect("bound frames").body(),
            charge: &mut self.content.charge,
            steps: &mut self.content.steps,
            maximum_steps: self.content.maximum_steps,
        }
    }
    pub(super) fn placed_header_flow(
        &mut self,
        variants: &[BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>],
        fragment: usize,
    ) -> Result<&'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        if variants.is_empty() {
            return Ok(self.content.flow);
        }
        for _ in 0..(u64::from(variants.len().checked_ilog2().unwrap_or(0)) + 2) {
            self.content.step(NodeId::new(0))?;
        }
        Ok(variants
            .binary_search_by_key(&fragment, |v| v.fragment)
            .map_or(self.content.flow, |i| variants[i].header.variant().flow()))
    }
    pub(super) fn place_variant_page_markers(
        &mut self,
        fragments: &[ProductionBodyFootnotePlacedFragment],
        variants: &[BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>],
        repetitions: impl Iterator<Item = bool>,
    ) -> Result<
        (
            Vec<ProductionBodyListMarker>,
            Vec<ProductionBodyFootnotePlacedMarker>,
        ),
        ProductionBodyPaginationError,
    > {
        let mut lists = Vec::new();
        let mut notes = Vec::new();
        for (index, (fragment, repeated)) in fragments.iter().zip(repetitions).enumerate() {
            let flow = self.placed_header_flow(variants, index)?;
            self.content_placement_for(flow)
                .place_fragment_markers(index, fragment, repeated, &mut lists, &mut notes)?;
        }
        Ok((lists, notes))
    }
    pub(super) fn translate_variant_page_origins(
        &mut self,
        selection: &BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
        fragments: &mut [ProductionBodyFootnotePlacedFragment],
        variants: &[BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>],
        lists: &mut [ProductionBodyListMarker],
        notes: &mut [ProductionBodyFootnotePlacedMarker],
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        for (index, placed) in fragments.iter_mut().enumerate() {
            let flow = self.placed_header_flow(variants, index)?;
            let owner = placed.fragment.owner;
            self.content.step(owner)?;
            let delta = origin_delta(flow, selection, placed.definition.is_some())?;
            placed.fragment.bounds = translate_page_rect_x(placed.fragment.bounds, delta, owner)?;
            placed.fragment.viewport = placed
                .fragment
                .viewport
                .map(|r| translate_page_rect_x(r, delta, owner))
                .transpose()?;
        }
        for marker in lists {
            let index = marker.fragment_index() as usize;
            let flow = self.placed_header_flow(variants, index)?;
            let placed = fragments
                .get(index)
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            self.content.step(marker.owner())?;
            marker.translate_x(origin_delta(flow, selection, placed.definition.is_some())?)?;
        }
        for marker in notes {
            let index = marker.fragment_index() as usize;
            let flow = self.placed_header_flow(variants, index)?;
            let placed = fragments
                .get(index)
                .filter(|p| p.definition == Some(marker.definition_index()))
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            self.content.step(placed.fragment.owner)?;
            marker.translate_x(origin_delta(flow, selection, true)?, placed.fragment.owner)?;
        }
        Ok(())
    }
}
pub(super) fn origin_delta(
    flow: &BookV2PreparedBodyFlow<'_, '_, '_, '_>,
    page: &BookV2BodyMixedPageSelection<'_, '_, '_, '_, '_>,
    note: bool,
) -> Result<Length, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let frames = flow
        .lines()
        .frames()
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let (actual, measured) = if note {
        (page.declared_footnote_region(), frames.footnote_region())
    } else {
        (Some(page.body_bounds()), Some(frames.body()))
    };
    match (actual, measured) {
        (Some(a), Some(m)) => a
            .x()
            .checked_sub(m.x())
            .ok_or_else(|| error(root, E::ArithmeticOverflow)),
        (None, None) => Ok(Length::ZERO),
        _ => Err(error(root, E::ReceiptMismatch)),
    }
}
