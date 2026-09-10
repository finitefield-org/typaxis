//! Nonpainting source anchors. A paragraph without a selected line keeps its
//! unresolved marker; it must not acquire an invented destination coordinate.
use super::*;
use typaxis_pagination::{ProductionBodyFootnotePlacedFragment, ProductionTablePlacedCellRole};

pub const BOOK_V2_ANCHOR_DISPLAY_ALGORITHM: &str = "typaxis.book-2-anchor-display/1";

pub struct BookV2AnchorPosition<'g, 's> {
    anchor: &'s ProductionPlacedInlineAnchor,
    fragment: &'g ProductionBodyFootnotePlacedFragment,
    index: usize,
    cell: Option<ProductionTablePlacedCellRole>,
    repeated: bool,
    x: Length,
    baseline: Length,
}
impl<'g, 's> BookV2AnchorPosition<'g, 's> {
    pub fn anchor(&self) -> &'s ProductionPlacedInlineAnchor {
        self.anchor
    }
    pub fn fragment(&self) -> &'g ProductionBodyFootnotePlacedFragment {
        self.fragment
    }
    pub fn fragment_index(&self) -> usize {
        self.index
    }
    pub fn cell_role(&self) -> Option<ProductionTablePlacedCellRole> {
        self.cell
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
    pub fn x(&self) -> Length {
        self.x
    }
    pub fn baseline(&self) -> Length {
        self.baseline
    }
}
/// A selected source line with no text, vector or math paint. Its actual line
/// box participates in pagination and source destinations, without an MCID.
pub struct BookV2NonpaintingLine<'g> {
    fragment: &'g ProductionBodyFootnotePlacedFragment,
    index: usize,
    cell: Option<ProductionTablePlacedCellRole>,
    repeated: bool,
}
impl<'g> BookV2NonpaintingLine<'g> {
    pub fn fragment(&self) -> &'g ProductionBodyFootnotePlacedFragment {
        self.fragment
    }
    pub fn fragment_index(&self) -> usize {
        self.index
    }
    pub fn cell_role(&self) -> Option<ProductionTablePlacedCellRole> {
        self.cell
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
}
pub struct BookV2AnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    positions: Vec<BookV2AnchorPosition<'g, 's>>,
    nonpainting: Vec<BookV2NonpaintingLine<'g>>,
    unpositioned: Vec<&'s ProductionPlacedInlineAnchor>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2AnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn positions(&self) -> &[BookV2AnchorPosition<'g, 's>] {
        &self.positions
    }
    pub fn nonpainting_lines(&self) -> &[BookV2NonpaintingLine<'g>] {
        &self.nonpainting
    }
    /// These original markers have no selected line. Positioned anchors in
    /// unreferenced definitions remain on the explicit unpainted source flow.
    pub fn unpositioned(&self) -> &[&'s ProductionPlacedInlineAnchor] {
        &self.unpositioned
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    fn visit_nonpainting_lines(
        &mut self,
        mut emit: impl FnMut(&mut Self, BookV2NonpaintingLine<'g>) -> Result<(), BookV2MathDisplayError>,
    ) -> Result<(), BookV2MathDisplayError> {
        let mut index = 0usize;
        for page in self.source.source().geometry().pages() {
            self.step(NodeId::new(0))?;
            for (placed, role, repeated) in page.fragments_with_roles() {
                let fragment = placed.fragment();
                let owner = fragment.owner();
                self.step(owner)?;
                let lines = self
                    .source
                    .fragment_flow(index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    .lines();
                if let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                {
                    let paragraph = lines
                        .paragraphs()
                        .get(paragraph_index as usize)
                        .filter(|p| p.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let line = paragraph
                        .lines()
                        .get(line_index as usize)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let mut nonpainting = true;
                    for item in line.items() {
                        self.step(owner)?;
                        if !matches!(item, typaxis_layout::ProductionPlacedInline::Break(_)) {
                            nonpainting = false;
                            break;
                        }
                    }
                    if nonpainting {
                        let baseline = plus(fragment.bounds().y(), line.baseline(), owner)?;
                        if fragment.baseline() != Some(baseline) {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                        emit(
                            self,
                            BookV2NonpaintingLine {
                                fragment: placed,
                                index,
                                cell: role,
                                repeated,
                            },
                        )?;
                    }
                }
                index = index
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::RecordLimit))?;
            }
        }
        Ok(())
    }
    fn visit_anchor_positions(
        &mut self,
        mut emit: impl FnMut(
            &mut Self,
            BookV2AnchorPosition<'g, 's>,
        ) -> Result<(), BookV2MathDisplayError>,
    ) -> Result<(), BookV2MathDisplayError> {
        let mut index = 0usize;
        for page in self.source.source().geometry().pages() {
            self.step(NodeId::new(0))?;
            for (placed, role, repeated) in page.fragments_with_roles() {
                let fragment = placed.fragment();
                let owner = fragment.owner();
                self.step(owner)?;
                let lines = self
                    .source
                    .fragment_flow(index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    .lines();
                if let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                {
                    let paragraph = lines
                        .paragraphs()
                        .get(paragraph_index as usize)
                        .filter(|p| p.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    // Bound lookup work and avoid rescanning a chapter's whole
                    // anchor list for each selected line or repeated header.
                    for _ in 0..usize::BITS - paragraph.anchors().len().leading_zeros() {
                        self.step(owner)?;
                    }
                    let first = paragraph.anchors().partition_point(|a| {
                        a.position().is_some_and(|p| p.line_index() < line_index)
                    });
                    for anchor in &paragraph.anchors()[first..] {
                        let owner = anchor.source().owner();
                        self.step(owner)?;
                        let local = anchor
                            .position()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if local.line_index() != line_index {
                            break;
                        }
                        let baseline = plus(fragment.bounds().y(), local.baseline(), owner)?;
                        if fragment.baseline() != Some(baseline) {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                        emit(
                            self,
                            BookV2AnchorPosition {
                                anchor,
                                fragment: placed,
                                index,
                                cell: role,
                                repeated,
                                x: plus(fragment.bounds().x(), local.x(), owner)?,
                                baseline,
                            },
                        )?;
                    }
                }
                index = index
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::RecordLimit))?;
            }
        }
        Ok(())
    }
    fn anchor_hash(
        &mut self,
        fp: [u8; 32],
        anchor: &ProductionPlacedInlineAnchor,
    ) -> Result<[u8; 32], BookV2MathDisplayError> {
        let source = anchor.source();
        let span = source.source_span();
        let mut bytes = [0; 20];
        for (slot, value) in bytes.chunks_exact_mut(4).zip([
            source.owner().get(),
            span.source_id().get(),
            span.start_byte().get(),
            span.end_byte().get(),
            source.boundary_unit(),
        ]) {
            slot.copy_from_slice(&value.to_be_bytes());
        }
        self.fold(fp, &bytes, source.owner())
    }
    pub fn build_anchors(
        &mut self,
    ) -> Result<BookV2AnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let lines = self.source.source().flow().lines();
        let mut unresolved = 0usize;
        for p in lines.paragraphs() {
            self.step(p.owner())?;
            for a in p.anchors() {
                self.step(a.source().owner())?;
                if a.position().is_none() {
                    unresolved = unresolved
                        .checked_add(1)
                        .ok_or_else(|| error(root, E::RecordLimit))?;
                }
            }
        }
        let mut count = 0usize;
        self.visit_anchor_positions(|_, _| {
            count = count
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?;
            Ok(())
        })?;
        let mut empty_count = 0usize;
        self.visit_nonpainting_lines(|_, _| {
            empty_count = empty_count
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?;
            Ok(())
        })?;
        let required = count
            .checked_add(unresolved)
            .and_then(|n| n.checked_add(empty_count))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| error(root, E::RecordLimit))?;
        take(&mut self.remaining, required, root)?;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut unpositioned = Vec::new();
        unpositioned
            .try_reserve_exact(unresolved)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut nonpainting = Vec::new();
        nonpainting
            .try_reserve_exact(empty_count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        self.step(root)?;
        let mut fp = sha256(BOOK_V2_ANCHOR_DISPLAY_ALGORITHM.as_bytes());
        fp = self.fold(fp, &self.source.fingerprint(), root)?;
        fp = self.fold(fp, &(count as u64).to_be_bytes(), root)?;
        self.visit_anchor_positions(|this, position| {
            let owner = position.anchor.source().owner();
            fp = this.anchor_hash(fp, position.anchor)?;
            let mut bytes = [0; 51];
            bytes[..4].copy_from_slice(&position.fragment.fragment().page_index().to_be_bytes());
            bytes[4..12].copy_from_slice(&(position.index as u64).to_be_bytes());
            bytes[12..20].copy_from_slice(&position.x.raw().to_be_bytes());
            bytes[20..28].copy_from_slice(&position.baseline.raw().to_be_bytes());
            bytes[28..36].copy_from_slice(&(position.fragment.item_index() as u64).to_be_bytes());
            bytes[36..44].copy_from_slice(
                &(position.fragment.definition_index().unwrap_or(0) as u64).to_be_bytes(),
            );
            bytes[44] = u8::from(position.fragment.definition_index().is_some());
            bytes[45..49].copy_from_slice(
                &position
                    .cell
                    .map(|c| c.owner().get())
                    .unwrap_or(0)
                    .to_be_bytes(),
            );
            bytes[49] = u8::from(position.cell.is_some());
            bytes[50] = u8::from(position.repeated);
            fp = this.fold(fp, &bytes, owner)?;
            positions.push(position);
            Ok(())
        })?;
        fp = self.fold(fp, &(unresolved as u64).to_be_bytes(), root)?;
        for p in lines.paragraphs() {
            self.step(p.owner())?;
            for anchor in p.anchors() {
                self.step(anchor.source().owner())?;
                if anchor.position().is_none() {
                    fp = self.anchor_hash(fp, anchor)?;
                    unpositioned.push(anchor);
                }
            }
        }
        fp = self.fold(fp, &(empty_count as u64).to_be_bytes(), root)?;
        self.visit_nonpainting_lines(|this, empty| {
            let f = empty.fragment.fragment();
            let bounds = f.bounds();
            let mut bytes = [0u8; 54];
            bytes[..4].copy_from_slice(&f.owner().get().to_be_bytes());
            bytes[4..8].copy_from_slice(&f.page_index().to_be_bytes());
            bytes[8..16].copy_from_slice(&(empty.index as u64).to_be_bytes());
            for (slot, value) in bytes[16..48].chunks_exact_mut(8).zip([
                bounds.x().raw(),
                bounds.y().raw(),
                bounds.width().get().raw(),
                bounds.height().get().raw(),
            ]) {
                slot.copy_from_slice(&value.to_be_bytes());
            }
            bytes[48..52].copy_from_slice(
                &empty
                    .cell
                    .map(|c| c.owner().get())
                    .unwrap_or(0)
                    .to_be_bytes(),
            );
            bytes[52] = u8::from(empty.cell.is_some());
            bytes[53] = u8::from(empty.repeated);
            fp = this.fold(fp, &bytes, f.owner())?;
            nonpainting.push(empty);
            Ok(())
        })?;
        Ok(BookV2AnchorDisplay {
            source: self.source,
            positions,
            nonpainting,
            unpositioned,
            fingerprint: fp,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }
}
