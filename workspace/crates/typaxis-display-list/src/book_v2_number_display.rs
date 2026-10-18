//! Independently authored numbers retain their actual successor shape and page.
pub use super::super::number_geometry::ProductionEquationNumberCluster;
use super::*;
use typaxis_pagination::book_v2::BookV2BodyPlacedEquationNumber;
use typaxis_shaping::ProductionBodyFont;
use typaxis_shaping::book_v2::{BookV2EquationNumberShape, book_v2_equation_number_font};

pub const BOOK_V2_NUMBER_DISPLAY_ALGORITHM: &str = "typaxis.book-2-number-display/1";

pub struct BookV2EquationNumberDraw<'g, 's, 'a> {
    placement: &'g BookV2BodyPlacedEquationNumber,
    shape: &'s BookV2EquationNumberShape<'a>,
    font: ProductionBodyFont,
    clusters: Vec<ProductionEquationNumberCluster<'a>>,
}
impl<'g, 's, 'a> BookV2EquationNumberDraw<'g, 's, 'a> {
    pub fn placement(&self) -> &'g BookV2BodyPlacedEquationNumber {
        self.placement
    }
    pub fn shape(&self) -> &'s BookV2EquationNumberShape<'a> {
        self.shape
    }
    pub fn font(&self) -> &ProductionBodyFont {
        &self.font
    }
    pub fn clusters(&self) -> &[ProductionEquationNumberCluster<'a>] {
        &self.clusters
    }
}
pub type BookV2EquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2SourceEquationNumberDisplay<
        'd,
        'g,
        's,
        'a,
        BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    >;
/// A column component retains its distinct terminal source owner.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2ColumnEquationNumberDisplay, BookV2EquationNumberDisplay};
/// fn single<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
///     value: BookV2ColumnEquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
/// ) -> BookV2EquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> { value }
/// ```
pub type BookV2ColumnEquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2SourceEquationNumberDisplay<
        'd,
        'g,
        's,
        'a,
        typaxis_pagination::book_v2::BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    >;

pub(super) struct EquationNumberProjection<'g, 's, 'a> {
    pub(super) draws: Vec<BookV2EquationNumberDraw<'g, 's, 'a>>,
    pub(super) fingerprint: [u8; 32],
}
pub struct BookV2SourceEquationNumberDisplay<'d, 'g, 's, 'a, S: ?Sized> {
    source: &'d S,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    draws: Vec<BookV2EquationNumberDraw<'g, 's, 'a>>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 's, 'a, S: ?Sized> BookV2SourceEquationNumberDisplay<'d, 'g, 's, 'a, S> {
    pub(super) fn from_projection(
        source: &'d S,
        admitted: &'d AdmittedProductionResourceLedgerV3,
        projection: EquationNumberProjection<'g, 's, 'a>,
        records: u64,
        work: u64,
    ) -> Self {
        Self {
            source,
            admitted,
            draws: projection.draws,
            fingerprint: projection.fingerprint,
            records,
            work,
        }
    }
    pub fn source(&self) -> &'d S {
        self.source
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.admitted
    }
    pub fn draws(&self) -> &[BookV2EquationNumberDraw<'g, 's, 'a>] {
        &self.draws
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
impl<'e, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> Projector<'e, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    fn number_shape(
        &mut self,
        placement: &BookV2BodyPlacedEquationNumber,
    ) -> Result<&'s BookV2EquationNumberShape<'a>, BookV2MathDisplayError> {
        let geometry = placement.geometry();
        let owner = geometry.owner();
        let numbers = self
            .source
            .fragment_graph(geometry.fragment_index() as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
            .blocks()
            .and_then(|b| b.numbers())
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        for _ in 0..usize::BITS - numbers.shapes().len().leading_zeros() + 1 {
            self.step(owner)?;
        }
        let shape = numbers
            .shape(geometry.parent_owner())
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        if shape.source().equation_number().map(|n| n.node_id()) != Some(owner)
            || shape.fingerprint() != geometry.shape_fingerprint()
            || shape.width() != geometry.bounds().width()
            || shape.height() != geometry.bounds().height()
        {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        Ok(shape)
    }
    fn number_records(
        &mut self,
        shape: &BookV2EquationNumberShape<'_>,
    ) -> Result<(usize, usize), BookV2MathDisplayError> {
        let owner = shape.source().node_id();
        let overflow = || error(owner, E::RecordLimit);
        let mut required = shape.runs().len().checked_mul(3).ok_or_else(overflow)?;
        let mut count = 0usize;
        for run in shape.runs() {
            self.step(owner)?;
            required = required
                .checked_add(run.glyphs().len())
                .ok_or_else(overflow)?;
            for cluster in run.clusters() {
                self.step(owner)?;
                let glyphs = cluster
                    .glyph_end
                    .checked_sub(cluster.glyph_start)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    as usize;
                required = required
                    .checked_add(1)
                    .and_then(|n| n.checked_add(glyphs))
                    .ok_or_else(overflow)?;
                count = count.checked_add(1).ok_or_else(overflow)?;
            }
        }
        Ok((required, count))
    }
    pub(super) fn collect_equation_numbers(
        &mut self,
    ) -> Result<EquationNumberProjection<'g, 's, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let source = self.source;
        let pages = source.pages();
        let mut required = 1usize;
        let mut count = 0usize;
        for page in pages {
            self.step(root)?;
            for placement in page.equation_numbers() {
                let shape = self.number_shape(placement)?;
                let (records, _) = self.number_records(shape)?;
                required = required
                    .checked_add(1)
                    .and_then(|n| n.checked_add(records))
                    .ok_or_else(|| error(root, E::RecordLimit))?;
                count = count
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::RecordLimit))?;
            }
        }
        // All retained clusters and temporary run/glyph arrays are reserved
        // before allocation. Neither successful disposal nor failure refunds it.
        take(&mut *self.remaining, required, root)?;
        let mut draws = Vec::new();
        draws
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        self.step(root)?;
        let mut fingerprint = sha256(BOOK_V2_NUMBER_DISPLAY_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &source.fingerprint(), root)?;
        fingerprint = self.fold(fingerprint, &self.admitted.fingerprint(), root)?;
        for page in pages {
            self.step(root)?;
            for placement in page.equation_numbers() {
                let shape = self.number_shape(placement)?;
                let geometry = placement.geometry();
                let owner = geometry.owner();
                let (records, count) = self.number_records(shape)?;
                let mut reserved = records as u64;
                self.step(owner)?;
                let font = book_v2_equation_number_font(shape, self.admitted)
                    .map_err(|_| error(owner, E::ReceiptMismatch))?;
                let mut clusters = Vec::new();
                clusters
                    .try_reserve_exact(count)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                number_geometry::project_number(
                    number_geometry::NumberSource {
                        owner,
                        text_span: shape
                            .source()
                            .equation_number()
                            .expect("verified number source")
                            .text()
                            .text_span(),
                        text: shape.text(),
                        runs: shape.runs(),
                    },
                    &font,
                    geometry.bounds(),
                    &mut reserved,
                    |n| self.step(n),
                    |cluster| {
                        clusters.push(cluster);
                        Ok(())
                    },
                )?;
                if reserved != 0 || clusters.len() != count {
                    return Err(error(owner, E::ReceiptMismatch).into());
                }
                let mut bytes = [0u8; 53];
                bytes[..4].copy_from_slice(&owner.get().to_be_bytes());
                bytes[4..8].copy_from_slice(&geometry.parent_owner().get().to_be_bytes());
                bytes[8..12].copy_from_slice(&geometry.page_index().to_be_bytes());
                bytes[12..16].copy_from_slice(&geometry.fragment_index().to_be_bytes());
                put_rect(&mut bytes[16..48], geometry.bounds());
                bytes[48] = u8::from(placement.repeated_header());
                bytes[49..53].copy_from_slice(&shape.font_instance_id().get().to_be_bytes());
                fingerprint = self.fold(fingerprint, &bytes, owner)?;
                fingerprint = self.fold(fingerprint, &shape.fingerprint(), owner)?;
                let mut metrics = [0u8; 24];
                for (i, n) in [font.ascender(), font.descender(), font.line_gap()]
                    .into_iter()
                    .enumerate()
                {
                    metrics[i * 8..i * 8 + 8].copy_from_slice(&n.raw().to_be_bytes());
                }
                fingerprint = self.fold(fingerprint, &metrics, owner)?;
                for cluster in &clusters {
                    let span = cluster.text_span();
                    let mut bytes = [0u8; 37];
                    bytes[..4].copy_from_slice(&cluster.run_index().to_be_bytes());
                    bytes[4..8].copy_from_slice(&cluster.cluster_index().to_be_bytes());
                    bytes[8..12].copy_from_slice(&span.text_id().get().to_be_bytes());
                    bytes[12..16].copy_from_slice(&span.range().start_byte().get().to_be_bytes());
                    bytes[16..20].copy_from_slice(&span.range().end_byte().get().to_be_bytes());
                    bytes[20..28]
                        .copy_from_slice(&(cluster.exact_text().len() as u64).to_be_bytes());
                    bytes[28..36].copy_from_slice(&(cluster.glyphs().len() as u64).to_be_bytes());
                    bytes[36] = u8::from(cluster.logical_bounds().is_some());
                    fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    if let Some(bounds) = cluster.logical_bounds() {
                        let mut bytes = [0u8; 32];
                        put_rect(&mut bytes, bounds);
                        fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    }
                    for chunk in cluster.exact_text().as_bytes().chunks(104) {
                        fingerprint = self.fold(fingerprint, chunk, owner)?;
                    }
                    for glyph in cluster.glyphs() {
                        let mut bytes = [0u8; 18];
                        bytes[..2].copy_from_slice(&glyph.original_gid().get().to_be_bytes());
                        bytes[2..10].copy_from_slice(&glyph.x().raw().to_be_bytes());
                        bytes[10..18].copy_from_slice(&glyph.y().raw().to_be_bytes());
                        fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    }
                }
                draws.push(BookV2EquationNumberDraw {
                    placement,
                    shape,
                    font,
                    clusters,
                });
            }
        }
        Ok(EquationNumberProjection { draws, fingerprint })
    }
}

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn build_equation_numbers(
        &mut self,
    ) -> Result<BookV2EquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError>
    {
        let projection = self.projection().collect_equation_numbers()?;
        Ok(BookV2EquationNumberDisplay::from_projection(
            self.source,
            self.admitted,
            projection,
            self.record_charge(),
            self.work_steps(),
        ))
    }
}
