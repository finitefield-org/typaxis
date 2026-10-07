//! Join all original selected page regions before resource selection.
use super::*;
use typaxis_syntax::book_v2::{select_book_v2_page_master, BookV2PageRegionKind};

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    /// Consume the complete page/header/footer-ordered set. Missing, duplicate,
    /// foreign or unselected regions cannot be attached. Empty authored regions
    /// retain their owner even though they add no glyph paints.
    pub fn with_page_regions(
        self,
        regions: Vec<BookV2PageRegionDisplay<'d>>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_work: u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        self.with_page_regions_counted(
            regions,
            limits,
            maximum_work,
            prior_records,
            prior_work,
            &mut 0,
            &mut 0,
        )
    }
    /// Return the cumulative counters even when validation or merging fails.
    /// The output counters include existing body/region prefixes, not just join work.
    #[allow(clippy::too_many_arguments)]
    pub fn with_page_regions_counted(
        mut self,
        regions: Vec<BookV2PageRegionDisplay<'d>>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        let mut work = self.work;
        let mut records = self.records;
        let result = (|| {
            let owner = NodeId::new(0);
            self.verify_resources(self.admitted(), limits)?;
            if !self.regions.is_empty() {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
            let source = self
                .source()
                .source()
                .flow()
                .lines()
                .prepared()
                .source_flow()
                .body();
            let epoch = self
                .source()
                .source()
                .flow()
                .lines()
                .prepared()
                .shaped()
                .binding_epoch();
            let pages = self.source().source().geometry().pages();
            for region in &regions {
                work = work
                    .checked_add(region.projection_work())
                    .ok_or(BookV2MathDisplayError::WorkLimit(owner))?;
                records = records
                    .checked_add(region.owned_records())
                    .ok_or_else(|| error(owner, E::RecordLimit))?;
            }
            work = work.max(prior_work);
            records = records.max(prior_records);
            if records > limits.base().get().max_fragments {
                return Err(error(owner, E::RecordLimit).into());
            }
            step(&mut work, maximum_work, 1)?;
            let mut cursor = 0;
            let mut additional = 0usize;
            for (page_index, page) in pages.iter().enumerate() {
                step(&mut work, maximum_work, 1)?;
                // Selection owns its own precise rule/master scan budget.
                let selected = select_book_v2_page_master(
                    source,
                    page_index as u32,
                    page.selection().named_page(),
                    &mut work,
                    maximum_work,
                )
                .map_err(|_| error(owner, E::ReceiptMismatch))?;
                for (kind, content, rect) in [
                    (
                        BookV2PageRegionKind::Header,
                        selected.advanced().header_content.as_ref(),
                        selected.master().header,
                    ),
                    (
                        BookV2PageRegionKind::Footer,
                        selected.advanced().footer_content.as_ref(),
                        selected.master().footer,
                    ),
                ] {
                    step(&mut work, maximum_work, 1)?;
                    let Some(content) = content else { continue };
                    let region = regions
                        .get(cursor)
                        .ok_or_else(|| error(NodeId::new(content.node_id), E::ReceiptMismatch))?;
                    region.verify_resources(source, self.admitted(), limits, epoch)?;
                    let rect = rect.ok_or_else(|| error(region.owner(), E::ReceiptMismatch))?;
                    let f = region.frame();
                    if region.owner().get() != content.node_id
                        || region.kind() != kind
                        || region.page_index() != page_index as u32
                        || [
                            f.x().raw(),
                            f.y().raw(),
                            f.width().get().raw(),
                            f.height().get().raw(),
                        ] != [rect.x, rect.y, rect.width, rect.height]
                    {
                        return Err(error(region.owner(), E::ReceiptMismatch).into());
                    }
                    additional = additional
                        .checked_add(region.draws().len())
                        .ok_or_else(|| error(owner, E::RecordLimit))?;
                    cursor += 1;
                }
            }
            if cursor != regions.len() {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
            if regions.is_empty() {
                // An unselected master can still carry authored regions. Keep the
                // caller's charges and the actual master scan even without paints.
                self.records = records;
                self.work = work;
                return Ok(self);
            }
            let count = self
                .paints
                .len()
                .checked_add(additional)
                .ok_or_else(|| error(owner, E::RecordLimit))?;
            // Old paint storage coexists during the merge; reserve the complete new
            // vector and region-owner slots before allocating it.
            records = records
                .checked_add(count as u64)
                .and_then(|n| n.checked_add(regions.len() as u64 + 1))
                .filter(|n| *n <= limits.base().get().max_fragments)
                .ok_or_else(|| error(owner, E::RecordLimit))?;
            let mut paints = Vec::new();
            paints
                .try_reserve_exact(count)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            let mut body_cursor = 0;
            let mut region_cursor = 0;
            for page in 0..pages.len() {
                while let Some(&paint) = self.paints.get(body_cursor) {
                    step(&mut work, maximum_work, 1)?;
                    let actual = self.paint_page(paint);
                    if actual > page as u32 {
                        break;
                    }
                    if actual != page as u32 {
                        return Err(error(owner, E::ReceiptMismatch).into());
                    }
                    paints.push(paint);
                    body_cursor += 1;
                }
                while let Some(region) = regions
                    .get(region_cursor)
                    .filter(|r| r.page_index() == page as u32)
                {
                    step(&mut work, maximum_work, 1)?;
                    for draw in 0..region.draws().len() {
                        step(&mut work, maximum_work, 1)?;
                        paints.push(BookV2BodyPaintIndex::PageRegionText {
                            region: region_cursor,
                            draw,
                        });
                    }
                    region_cursor += 1;
                }
            }
            if body_cursor != self.paints.len()
                || region_cursor != regions.len()
                || paints.len() != count
            {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
            let mut fingerprint = fold(self.fingerprint, b"typaxis.book-2-page-region-join/1");
            step(&mut work, maximum_work, 1)?;
            for region in &regions {
                step(&mut work, maximum_work, 1)?;
                fingerprint = fold(fingerprint, &region.fingerprint());
            }
            self.paints = paints;
            self.regions = regions;
            self.fingerprint = fingerprint;
            self.records = records;
            self.work = work;
            Ok(self)
        })();
        *observed_records = records;
        *observed_work = work;
        result
    }
    fn paint_page(&self, paint: BookV2BodyPaintIndex) -> u32 {
        use BookV2BodyPaintIndex as P;
        match paint {
            P::Text(i) => self.text().draws()[i].page_index(),
            P::Marker(i) => self.markers().draws()[i].fragment().fragment().page_index(),
            P::Math(i) => self.math().draws()[i].terminal().page_index(),
            P::Image(i) => self.images().draws()[i].fragment().fragment().page_index(),
            P::EquationNumber(i) => self.numbers().draws()[i]
                .placement()
                .geometry()
                .page_index(),
            P::FootnoteSeparator(i) => self.markers().separators()[i].page_index(),
            P::PageRegionText { region, .. } => self.page_regions()[region].page_index(),
        }
    }
}

fn step(work: &mut u64, maximum: u64, count: u64) -> Result<(), BookV2MathDisplayError> {
    *work = work
        .checked_add(count)
        .filter(|n| *n <= maximum)
        .ok_or(BookV2MathDisplayError::WorkLimit(NodeId::new(0)))?;
    Ok(())
}
