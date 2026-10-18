//! Complete column paints retain their own closed terminal source.
use super::*;
use typaxis_pagination::book_v2::BookV2ColumnMathTerminals;
#[path = "book_v2_column_page_region_join.rs"]
mod column_page_region_join;

pub const BOOK_V2_COLUMN_DISPLAY_ALGORITHM: &str = "typaxis.book-2-column-display/1";

/// Actual inputs to one successful consuming running-region join. These
/// bounded scalars share the already charged display owner and allocate no
/// document-sized history. They do not reconstruct source preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2ColumnPageRegionJoinObservation {
    base_fingerprint: [u8; 32],
    base_records: u64,
    base_work: u64,
    prior_records: u64,
    prior_work: u64,
}
impl BookV2ColumnPageRegionJoinObservation {
    pub fn base_fingerprint(self) -> [u8; 32] {
        self.base_fingerprint
    }
    pub fn base_records(self) -> u64 {
        self.base_records
    }
    pub fn base_work(self) -> u64 {
        self.base_work
    }
    pub fn prior_records(self) -> u64 {
        self.prior_records
    }
    pub fn prior_work(self) -> u64 {
        self.prior_work
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ColumnPaintIndex {
    FootnoteSeparator(usize),
    Marker(usize),
    Text(usize),
    Math(usize),
    Image(usize),
    EquationNumber(usize),
    /// O(1) private region/draw locator; preserves the original paint footprint.
    PageRegionText {
        region_draw: usize,
    },
}

/// Column display ownership cannot authorize the body PDF path.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2ColumnDisplay, BookV2BodyDisplay};
/// fn body<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(v: BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    math: BookV2ColumnMathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    text: BookV2ColumnTextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    numbers: BookV2ColumnEquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    markers: BookV2ColumnMarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    images: BookV2ColumnImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    anchors: BookV2ColumnAnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    paints: Vec<BookV2ColumnPaintIndex>,
    fragment_columns: Vec<Option<u16>>,
    regions: Vec<BookV2PageRegionDisplay<'d>>,
    region_draws: Vec<(usize, usize)>,
    region_join: Option<BookV2ColumnPageRegionJoinObservation>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.math.source()
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.math.admitted()
    }
    pub fn math(&self) -> &BookV2ColumnMathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.math
    }
    pub fn text(&self) -> &BookV2ColumnTextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.text
    }
    pub fn numbers(&self) -> &BookV2ColumnEquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.numbers
    }
    pub fn markers(&self) -> &BookV2ColumnMarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.markers
    }
    pub fn images(&self) -> &BookV2ColumnImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.images
    }
    pub fn anchors(&self) -> &BookV2ColumnAnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.anchors
    }
    pub fn paints(&self) -> &[BookV2ColumnPaintIndex] {
        &self.paints
    }
    pub fn page_regions(&self) -> &[BookV2PageRegionDisplay<'d>] {
        &self.regions
    }
    pub fn page_region_join_observation(&self) -> Option<BookV2ColumnPageRegionJoinObservation> {
        self.region_join
    }
    pub(crate) fn page_region_paint(
        &self,
        paint: BookV2ColumnPaintIndex,
    ) -> Option<(usize, usize)> {
        match paint {
            BookV2ColumnPaintIndex::PageRegionText { region_draw } => {
                self.region_draws.get(region_draw).copied()
            }
            _ => None,
        }
    }
    /// None denotes the shared note region, or an out-of-range index.
    pub fn fragment_column(&self, index: usize) -> Option<u16> {
        self.fragment_columns.get(index).copied().flatten()
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
    pub fn verify_resources(
        &self,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2MathDisplayError> {
        if !std::ptr::eq(self.admitted(), admitted) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
        }
        self.source()
            .original_graph()
            .verify(limits)
            .map_err(|_| error(NodeId::new(0), E::ReceiptMismatch))?;
        Ok(())
    }
    fn counts(&self) -> [usize; 6] {
        [
            self.markers.separators().len(),
            self.markers.draws().len(),
            self.text.draws().len(),
            self.math.draws().len(),
            self.images.draws().len(),
            self.numbers.draws().len(),
        ]
    }
    fn next(&self, stream: usize, index: usize) -> (BookV2ColumnPaintIndex, (usize, u8, u32)) {
        use BookV2ColumnPaintIndex as P;
        match stream {
            0 => (
                P::FootnoteSeparator(index),
                (
                    self.markers.separators()[index].before_fragment_index(),
                    0,
                    0,
                ),
            ),
            1 => (
                P::Marker(index),
                (self.markers.draws()[index].fragment_index(), 1, 0),
            ),
            2 => {
                let d = &self.text.draws()[index];
                (P::Text(index), (d.fragment_index(), 2, d.inline_index()))
            }
            3 => {
                let t = self.math.draws()[index].terminal();
                (
                    P::Math(index),
                    (t.fragment_index(), 2, t.inline_index().unwrap_or(0)),
                )
            }
            4 => {
                let d = &self.images.draws()[index];
                (
                    P::Image(index),
                    (d.fragment_index(), 2, d.inline_index().unwrap_or(0)),
                )
            }
            5 => (
                P::EquationNumber(index),
                (
                    self.numbers.draws()[index]
                        .placement()
                        .geometry()
                        .fragment_index() as usize,
                    3,
                    0,
                ),
            ),
            _ => unreachable!("six column paint streams"),
        }
    }
}

pub struct BookV2ColumnDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    maximum_records: u64,
    remaining: u64,
    maximum_work: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn new(
        source: &'d BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        admitted: &'d AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_work: u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        Self::new_counted(
            source,
            admitted,
            limits,
            maximum_work,
            prior_records,
            prior_work,
            &mut 0,
            &mut 0,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new_counted(
        source: &'d BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        admitted: &'d AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        *observed_records = prior_records.max(source.record_charge());
        *observed_work = prior_work.max(source.work_steps());
        verify_graph(source.original_graph(), admitted, limits)?;
        let mut remaining = limits
            .base()
            .get()
            .max_fragments
            .checked_sub(*observed_records)
            .ok_or_else(|| error(root, E::RecordLimit))?;
        if *observed_work > maximum_work {
            return Err(BookV2MathDisplayError::WorkLimit(root));
        }
        take(&mut remaining, 1, root)?;
        let mut result = Self {
            source,
            admitted,
            maximum_records: limits.base().get().max_fragments,
            remaining,
            maximum_work,
            work: *observed_work,
        };
        let validation = (|| {
            if source.source().has_header_variants() {
                for page in SourceView::Column(source).pages() {
                    result.step(root)?;
                    for graph in page.header_graphs() {
                        result.step(root)?;
                        verify_graph(graph, admitted, limits)?;
                    }
                }
            }
            Ok::<(), BookV2MathDisplayError>(())
        })();
        *observed_records = result.record_charge();
        *observed_work = result.work_steps();
        validation?;
        Ok(result)
    }
    /// Continue after this exact display has consumed source-owned running regions.
    /// The typed receipt cannot inject a fresh counter or change physical family.
    pub fn continue_after_display(
        &mut self,
        display: &BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), BookV2MathDisplayError> {
        let owner = NodeId::new(0);
        if !std::ptr::eq(self.source, display.source())
            || !std::ptr::eq(self.admitted, display.admitted())
        {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        self.remaining = self
            .maximum_records
            .checked_sub(self.record_charge().max(display.record_charge()))
            .ok_or_else(|| error(owner, E::RecordLimit))?;
        self.work = self.work.max(display.work_steps());
        if self.work > self.maximum_work {
            return Err(BookV2MathDisplayError::WorkLimit(owner));
        }
        Ok(())
    }
    pub fn record_charge(&self) -> u64 {
        self.maximum_records - self.remaining
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    fn step(&mut self, owner: NodeId) -> Result<(), BookV2MathDisplayError> {
        self.work = self
            .work
            .checked_add(1)
            .filter(|n| *n <= self.maximum_work)
            .ok_or(BookV2MathDisplayError::WorkLimit(owner))?;
        Ok(())
    }
    pub(super) fn projection(&mut self) -> Projector<'_, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        Projector::new(
            SourceView::Column(self.source),
            self.admitted,
            &mut self.remaining,
            &mut self.work,
            self.maximum_work,
        )
    }
    pub fn build(
        &mut self,
    ) -> Result<BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let source = self.source;
        let math = self.projection().collect_math(source.terminals())?;
        let math = BookV2ColumnMathDisplay::from_projection(
            self.source,
            self.admitted,
            math,
            self.record_charge(),
            self.work_steps(),
        );
        let text = self.projection().collect_text()?;
        let text = BookV2ColumnTextDisplay::from_projection(
            self.source,
            self.admitted,
            text,
            self.record_charge(),
            self.work_steps(),
        );
        let numbers = self.projection().collect_equation_numbers()?;
        let numbers = BookV2ColumnEquationNumberDisplay::from_projection(
            self.source,
            self.admitted,
            numbers,
            self.record_charge(),
            self.work_steps(),
        );
        let markers = self.projection().collect_markers()?;
        let markers = BookV2ColumnMarkerDisplay::from_projection(
            self.source,
            self.admitted,
            markers,
            self.record_charge(),
            self.work_steps(),
        );
        let images = self.projection().collect_images()?;
        let images = BookV2ColumnImageDisplay::from_projection(
            self.source,
            self.admitted,
            images,
            self.record_charge(),
            self.work_steps(),
        );
        let anchors = self.projection().collect_anchors()?;
        let anchors = BookV2ColumnAnchorDisplay::from_projection(
            self.source,
            anchors,
            self.record_charge(),
            self.work_steps(),
        );
        let mut display = BookV2ColumnDisplay {
            math,
            text,
            numbers,
            markers,
            images,
            anchors,
            paints: Vec::new(),
            fragment_columns: Vec::new(),
            regions: Vec::new(),
            region_draws: Vec::new(),
            region_join: None,
            fingerprint: [0; 32],
            records: 0,
            work: 0,
        };
        let hashes = [
            self.source.fingerprint(),
            self.admitted.fingerprint(),
            display.math.fingerprint(),
            display.text.fingerprint(),
            display.numbers.fingerprint(),
            display.markers.fingerprint(),
            display.images.fingerprint(),
            display.anchors.fingerprint(),
        ];
        let (paints, mut fingerprint) = super::display_merge::merge(
            &mut self.projection(),
            BOOK_V2_COLUMN_DISPLAY_ALGORITHM,
            hashes,
            display.counts(),
            |stream, index| display.next(stream, index),
        )?;
        display.paints = paints;
        let count = self.source.fragment_count();
        take(
            &mut self.remaining,
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?,
            root,
        )?;
        display
            .fragment_columns
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for page in self.source.source().geometry().pages() {
            self.step(root)?;
            for (local, placed) in page.fragments().iter().enumerate() {
                let owner = placed.fragment().owner();
                self.step(owner)?;
                for _ in 0..usize::BITS - page.columns().len().leading_zeros() + 1 {
                    self.step(owner)?;
                }
                let column = page.fragment_column(local);
                if column.is_none() != placed.definition_index().is_some() {
                    return Err(error(owner, E::ReceiptMismatch).into());
                }
                display.fragment_columns.push(column);
                let mut bytes = [0; 3];
                bytes[0] = u8::from(column.is_some());
                bytes[1..].copy_from_slice(&column.unwrap_or(0).to_be_bytes());
                fingerprint = self.projection().fold(fingerprint, &bytes, owner)?;
            }
        }
        if display.fragment_columns.len() != count {
            return Err(error(root, E::ReceiptMismatch).into());
        }
        display.fingerprint = fingerprint;
        display.records = self.record_charge();
        display.work = self.work_steps();
        Ok(display)
    }
}

/// A scoped loan of the driver's original Column Display builder.
///
/// There is no constructor or mutable builder accessor. Extra builds retain
/// their accepted record/work prefix in that same builder, which its owner
/// reads after the callback returns normally, including an application error.
///
/// A genuine builder may issue another display within its scoped loan.
///
/// ```
/// use typaxis_display_list::book_v2::{BookV2ColumnDisplayBuilder as Builder, BookV2ColumnDisplay as Display, BookV2MathDisplayError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type D<'a> = Display<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn build<'a>(builder: &mut B<'a>) -> Result<D<'a>, BookV2MathDisplayError> {
///     builder.with_continuation(|loan| {
///         let _ = (loan.record_charge(), loan.work_steps());
///         loan.build()
///     })
/// }
/// ```
/// A raw builder cannot replace the continuation.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2ColumnDisplayBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn replace<'a>(builder: &mut B<'a>, fresh: B<'a>) {
///     builder.with_continuation(|loan| *loan = fresh);
/// }
/// ```
/// The private raw builder cannot be extracted.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2ColumnDisplayBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn raw<'a>(builder: &mut B<'a>) {
///     builder.with_continuation(|loan| { let _ = &mut loan.builder; });
/// }
/// ```
/// The continuation loan cannot escape its callback.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2ColumnDisplayBuilder as Builder, BookV2ColumnDisplayContinuation as Continuation};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn escape<'a>(builder: &'a mut B<'a>) -> &'a mut C<'a> {
///     builder.with_continuation(|loan| loan)
/// }
/// ```
/// Independent loans cannot exchange their invariant continuations.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2ColumnDisplayBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn swap<'a>(left: &mut B<'a>, right: &mut B<'a>) {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| std::mem::swap(outer, inner));
///     });
/// }
/// ```
/// Nested callbacks can inspect both genuine builders.
///
/// ```
/// use typaxis_display_list::book_v2::BookV2ColumnDisplayBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn inspect<'a>(left: &mut B<'a>, right: &mut B<'a>) -> u64 {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| outer.record_charge() + inner.record_charge())
///     })
/// }
/// ```
pub struct BookV2ColumnDisplayContinuation<'loan, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    builder: &'loan mut BookV2ColumnDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    invariant: std::marker::PhantomData<fn(&'loan ()) -> &'loan ()>,
}
impl<'loan, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ColumnDisplayContinuation<'loan, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn record_charge(&self) -> u64 {
        self.builder.record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.builder.work_steps()
    }
    pub fn build(
        &mut self,
    ) -> Result<BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        self.builder.build()
    }
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    /// Loan a nonreplaceable continuation while retaining this original builder.
    pub fn with_continuation<R>(
        &mut self,
        inspect: impl for<'loan> FnOnce(
            &mut BookV2ColumnDisplayContinuation<'loan, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        ) -> R,
    ) -> R {
        let mut continuation = BookV2ColumnDisplayContinuation {
            builder: self,
            invariant: std::marker::PhantomData,
        };
        inspect(&mut continuation)
    }
}

fn verify_graph(
    graph: typaxis_pagination::book_v2::BookV2SourceGraph<'_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), BookV2MathDisplayError> {
    let root = NodeId::new(0);
    graph
        .verify(limits)
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    let prepared = graph.lines().prepared();
    let shaped = prepared.shaped();
    shaped
        .verify(
            prepared.source_flow(),
            admitted,
            limits,
            shaped.binding_epoch(),
        )
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    if let Some(bindings) = prepared.vector_bindings() {
        bindings
            .verify(bindings.body(), admitted, limits)
            .map_err(|_| error(root, E::ReceiptMismatch))?;
    }
    Ok(())
}
