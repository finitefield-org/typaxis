//! Owned glyph projection outlives the temporary converged page-region layout.
//! Running regions are always page artifacts, including their first occurrence.
use super::*;
use typaxis_core::TextSpan;
use typaxis_layout::book_v2::BookV2ConvergedPageRegionLines;
use typaxis_resource_admission::{
    AdmittedProductionFontInstanceV3, AdmittedProductionFontInstancesV3,
};
use typaxis_shaping::{ProductionBodyFont, ShapeSourceSpan};
use typaxis_syntax::book_v2::{BookV2PageRegionKind, StyledBookV2Body};

pub const BOOK_V2_PAGE_REGION_DISPLAY_ALGORITHM: &str = "typaxis.book-2-page-region-display/1";

/// Sealed original-source cluster with copied page-space glyphs. There is no
/// body fragment, semantic node or MCID assigned to a running-region draw.
pub struct BookV2PageRegionTextDraw<'a> {
    owner: NodeId,
    paragraph: NodeId,
    line: u32,
    source: TextSpan,
    text: &'a str,
    font: ProductionBodyFont,
    instance: AdmittedProductionFontInstanceV3<'a>,
    bounds: Option<Rect>,
    glyphs: Vec<ProductionBodyGlyph>,
}
impl<'a> BookV2PageRegionTextDraw<'a> {
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn paragraph_owner(&self) -> NodeId {
        self.paragraph
    }
    pub fn line_index(&self) -> u32 {
        self.line
    }
    pub fn source_span(&self) -> TextSpan {
        self.source
    }
    pub fn exact_text(&self) -> &'a str {
        self.text
    }
    pub fn font(&self) -> &ProductionBodyFont {
        &self.font
    }
    pub fn font_instance(&self) -> AdmittedProductionFontInstanceV3<'a> {
        self.instance
    }
    pub fn logical_bounds(&self) -> Option<Rect> {
        self.bounds
    }
    pub fn glyphs(&self) -> &[ProductionBodyGlyph] {
        &self.glyphs
    }
}

/// Every draw in this owner is a Header/Footer pagination artifact. Resource
/// consumers must still select and encode these actual glyphs before PDF paint.
pub struct BookV2PageRegionDisplay<'a> {
    source: &'a StyledBookV2Body,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    kind: BookV2PageRegionKind,
    owner: NodeId,
    page: u32,
    frame: Rect,
    layout_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    epoch: [u8; 32],
    draws: Vec<BookV2PageRegionTextDraw<'a>>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
    owned_records: u64,
    projection_work: u64,
}
impl<'a> BookV2PageRegionDisplay<'a> {
    pub fn source(&self) -> &'a StyledBookV2Body {
        self.source
    }
    pub fn admitted(&self) -> &'a AdmittedProductionResourceLedgerV3 {
        self.admitted
    }
    pub fn kind(&self) -> BookV2PageRegionKind {
        self.kind
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn frame(&self) -> Rect {
        self.frame
    }
    pub fn draws(&self) -> &[BookV2PageRegionTextDraw<'a>] {
        &self.draws
    }
    pub fn layout_fingerprint(&self) -> [u8; 32] {
        self.layout_fingerprint
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
    pub(crate) fn owned_records(&self) -> u64 {
        self.owned_records
    }
    pub(crate) fn projection_work(&self) -> u64 {
        self.projection_work
    }
    pub fn verify_resources(
        &self,
        source: &StyledBookV2Body,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        epoch: [u8; 32],
    ) -> Result<(), BookV2MathDisplayError> {
        if !std::ptr::eq(self.source, source)
            || !std::ptr::eq(self.admitted, admitted)
            || self.limits_fingerprint != limits.fingerprint()
            || self.epoch != epoch
        {
            return Err(error(self.owner, E::ReceiptMismatch).into());
        }
        Ok(())
    }
}

/// One persistent counter owner for all requested pages/roles. Failed builds
/// retain consumed work and reserved output records. The caller accounts for
/// source/layout preparation separately; they are not paint authorization.
pub struct BookV2PageRegionDisplayBuilder<'a> {
    source: &'a StyledBookV2Body,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    instances: AdmittedProductionFontInstancesV3<'a>,
    limits: &'a M4EffectiveResourceLimits,
    epoch: [u8; 32],
    maximum_records: u64,
    remaining: u64,
    maximum_work: u64,
    work: u64,
}
impl<'a> BookV2PageRegionDisplayBuilder<'a> {
    pub fn new(
        source: &'a StyledBookV2Body,
        admitted: &'a AdmittedProductionResourceLedgerV3,
        limits: &'a M4EffectiveResourceLimits,
        epoch: [u8; 32],
        prior_records: u64,
        prior_work: u64,
        maximum_work: u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        Self::new_counted(
            source,
            admitted,
            limits,
            epoch,
            prior_records,
            prior_work,
            maximum_work,
            &mut 0,
            &mut 0,
        )
    }
    /// Preserve accepted reservations even if work admission or instance
    /// preparation fails before the persistent builder is returned.
    #[allow(clippy::too_many_arguments)]
    pub fn new_counted(
        source: &'a StyledBookV2Body,
        admitted: &'a AdmittedProductionResourceLedgerV3,
        limits: &'a M4EffectiveResourceLimits,
        epoch: [u8; 32],
        prior_records: u64,
        prior_work: u64,
        maximum_work: u64,
        observed_records: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        *observed_records = prior_records;
        *observed_work = prior_work;
        let owner = NodeId::new(0);
        if epoch == [0; 32]
            || admitted.effective_limits().fingerprint() != limits.fingerprint()
            || !admitted.matches_declared_resources(source.body().resources())
        {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        if prior_work > maximum_work {
            return Err(BookV2MathDisplayError::WorkLimit(owner));
        }
        let maximum_records = limits.base().get().max_fragments;
        let mut remaining = maximum_records
            .checked_sub(prior_records)
            .ok_or_else(|| error(owner, E::RecordLimit))?;
        // Retained table and its construction's face set/vector are prepaid.
        let slots = admitted
            .fonts()
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| error(owner, E::RecordLimit))?;
        take(&mut remaining, slots, owner)?;
        *observed_records = maximum_records - remaining;
        let work = prior_work
            .checked_add(slots as u64)
            .filter(|n| *n <= maximum_work)
            .ok_or(BookV2MathDisplayError::WorkLimit(owner))?;
        *observed_work = work;
        let instances = AdmittedProductionFontInstancesV3::from_used_faces(
            admitted,
            admitted.fonts().iter().map(|f| f.font_face_id()),
        )
        .map_err(|_| error(owner, E::ReceiptMismatch))?;
        Ok(Self {
            source,
            admitted,
            instances,
            limits,
            epoch,
            maximum_records,
            remaining,
            maximum_work,
            work,
        })
    }
    pub fn record_charge(&self) -> u64 {
        self.maximum_records - self.remaining
    }
    /// Carry intervening source/layout work into this persistent paint builder.
    pub fn continue_with_prior(
        &mut self,
        records: u64,
        work: u64,
    ) -> Result<(), BookV2MathDisplayError> {
        let owner = NodeId::new(0);
        self.remaining = self
            .maximum_records
            .checked_sub(records.max(self.record_charge()))
            .ok_or_else(|| error(owner, E::RecordLimit))?;
        self.work = self.work.max(work);
        if self.work > self.maximum_work {
            return Err(BookV2MathDisplayError::WorkLimit(owner));
        }
        Ok(())
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    fn steps(&mut self, count: u64, owner: NodeId) -> Result<(), BookV2MathDisplayError> {
        let Some(next) = self.work.checked_add(count) else {
            self.work = u64::MAX;
            return Err(BookV2MathDisplayError::WorkLimit(owner));
        };
        self.work = next;
        if self.work > self.maximum_work {
            return Err(BookV2MathDisplayError::WorkLimit(owner));
        }
        Ok(())
    }
    fn digest(
        &mut self,
        previous: [u8; 32],
        bytes: &[u8],
        owner: NodeId,
    ) -> Result<[u8; 32], BookV2MathDisplayError> {
        self.steps((32 + bytes.len()).div_ceil(64) as u64, owner)?;
        Ok(fold(previous, bytes))
    }
    pub fn build(
        &mut self,
        stable: &BookV2ConvergedPageRegionLines<'_, '_, '_>,
    ) -> Result<BookV2PageRegionDisplay<'a>, BookV2MathDisplayError> {
        let lines = stable.lines();
        let work_before = self.work;
        let flow = lines.prepared().flow();
        let owner = NodeId::new(flow.source().node_id);
        self.steps(1, owner)?;
        if !std::ptr::eq(flow.body(), self.source) {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        let shape = lines.prepared().shaped();
        shape
            .verify(flow, self.admitted, self.limits, self.epoch)
            .map_err(|_| error(owner, E::ReceiptMismatch))?;
        if shape.font_instances().fingerprint() != self.instances.fingerprint() {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        let mut slots = 1usize;
        let mut count = 0usize;
        for origin in lines.origins() {
            self.steps(1, owner)?;
            let paragraph = &lines.paragraphs()[origin.paragraph_index() as usize];
            let line = &paragraph.lines()[origin.line_index() as usize];
            for item in line.items() {
                self.steps(1, paragraph.owner())?;
                if let ProductionPlacedInline::Text(cluster) = item {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| error(owner, E::RecordLimit))?;
                    slots = slots
                        .checked_add(1)
                        .and_then(|n| n.checked_add(cluster.glyphs().len()))
                        .ok_or_else(|| error(owner, E::RecordLimit))?;
                }
            }
        }
        take(&mut self.remaining, slots, owner)?;
        let mut draws = Vec::new();
        draws
            .try_reserve_exact(count)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut fingerprint = sha256(BOOK_V2_PAGE_REGION_DISPLAY_ALGORITHM.as_bytes());
        for digest in [
            lines.fingerprint(),
            self.admitted.fingerprint(),
            self.limits.fingerprint(),
            self.epoch,
        ] {
            fingerprint = self.digest(fingerprint, &digest, owner)?;
        }
        for origin in lines.origins() {
            self.steps(1, owner)?;
            let paragraph = &lines.paragraphs()[origin.paragraph_index() as usize];
            let line = &paragraph.lines()[origin.line_index() as usize];
            for item in line.items() {
                self.steps(1, paragraph.owner())?;
                let ProductionPlacedInline::Text(cluster) = item else {
                    continue;
                };
                let node = cluster.run().owner();
                let font = *paragraph
                    .font()
                    .ok_or_else(|| error(node, E::ReceiptMismatch))?;
                let instance = self
                    .instances
                    .resolve(cluster.run().glyph_run().font)
                    .ok_or_else(|| error(node, E::ReceiptMismatch))?;
                if instance.font().font_face_id() != font.face_id()
                    || instance.font().content_hash() != font.content_hash()
                    || instance.font().face_index() != font.face_index()
                {
                    return Err(error(node, E::ReceiptMismatch).into());
                }
                let ShapeSourceSpan::Parsed(span) = cluster.source_span() else {
                    return Err(error(node, E::ReceiptMismatch).into());
                };
                let text = self
                    .source
                    .body()
                    .wire()
                    .text_buffers()
                    .get(span.text_id().get() as usize)
                    .and_then(|b| {
                        b.utf8
                            .get(span.start_byte().get() as usize..span.end_byte().get() as usize)
                    })
                    .ok_or_else(|| error(node, E::ReceiptMismatch))?;
                self.steps(text.len() as u64, node)?;
                if text != cluster.utf8() {
                    return Err(error(node, E::ReceiptMismatch).into());
                }
                self.steps(cluster.glyphs().len() as u64, node)?;
                let mut reserved = cluster.glyphs().len() as u64 + 1;
                let (glyphs, bounds) = text_geometry::project_cluster_at(
                    cluster,
                    &font,
                    origin.x(),
                    origin.y(),
                    Some(plus(origin.y(), line.baseline(), node)?),
                    &mut reserved,
                )?;
                let mut identity = [0u8; 32];
                for (chunk, n) in identity.chunks_exact_mut(4).zip([
                    node.get(),
                    paragraph.owner().get(),
                    origin.line_index(),
                    span.text_id().get(),
                    span.start_byte().get(),
                    span.end_byte().get(),
                    font.face_id().get(),
                    instance.font_instance_id().get(),
                ]) {
                    chunk.copy_from_slice(&n.to_be_bytes());
                }
                fingerprint = self.digest(fingerprint, &identity, node)?;
                for chunk in text.as_bytes().chunks(104) {
                    fingerprint = self.digest(fingerprint, chunk, node)?;
                }
                for glyph in &glyphs {
                    let mut bytes = [0u8; 18];
                    bytes[..2].copy_from_slice(&glyph.original_gid().get().to_be_bytes());
                    bytes[2..10].copy_from_slice(&glyph.x().raw().to_be_bytes());
                    bytes[10..].copy_from_slice(&glyph.y().raw().to_be_bytes());
                    fingerprint = self.digest(fingerprint, &bytes, node)?;
                }
                draws.push(BookV2PageRegionTextDraw {
                    owner: node,
                    paragraph: paragraph.owner(),
                    line: origin.line_index(),
                    source: span,
                    text,
                    font,
                    instance,
                    bounds,
                    glyphs,
                });
            }
        }
        Ok(BookV2PageRegionDisplay {
            source: self.source,
            admitted: self.admitted,
            kind: flow.kind(),
            owner,
            page: lines.selected_master().page_index(),
            frame: lines.frame(),
            layout_fingerprint: lines.fingerprint(),
            limits_fingerprint: self.limits.fingerprint(),
            epoch: self.epoch,
            draws,
            fingerprint,
            records: self.record_charge(),
            work: self.work,
            owned_records: slots as u64,
            projection_work: self.work - work_before,
        })
    }
}
