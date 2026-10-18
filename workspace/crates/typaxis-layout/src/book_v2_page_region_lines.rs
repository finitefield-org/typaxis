//! Source-bound running-region line geometry. No body or PDF paint authority.
use super::*;
use typaxis_core::Rect;
use typaxis_shaping::book_v2::BookV2PageRegionTextShape;
use typaxis_syntax::book_v2::{
    BookV2PageRegionKind, BookV2PageRegionTextFlow, BookV2SelectedPageMaster,
};

#[path = "book_v2_page_region_reshape.rs"]
mod reshape_region;
pub use reshape_region::*;

pub const BOOK_V2_PAGE_REGION_INLINE_ALGORITHM: &str = "typaxis.book-2-page-region-inlines/1";
pub const BOOK_V2_PAGE_REGION_LINE_ALGORITHM: &str = "typaxis.book-2-page-region-lines/1";

#[derive(Debug)]
pub enum BookV2PageRegionLayoutError {
    Inline(ProductionInlinePreparationError),
    Shape(typaxis_shaping::ProductionTextShapeError),
    Feedback(typaxis_linebreak::BreakError),
    Geometry {
        owner: NodeId,
    },
    Overflow {
        owner: NodeId,
        required: Length,
        available: PositiveLength,
    },
}
impl std::fmt::Display for BookV2PageRegionLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 page region: {self:?}")
    }
}
impl std::error::Error for BookV2PageRegionLayoutError {}
impl From<ProductionInlinePreparationError> for BookV2PageRegionLayoutError {
    fn from(e: ProductionInlinePreparationError) -> Self {
        Self::Inline(e)
    }
}
impl From<typaxis_shaping::ProductionTextShapeError> for BookV2PageRegionLayoutError {
    fn from(e: typaxis_shaping::ProductionTextShapeError) -> Self {
        Self::Shape(e)
    }
}
impl From<typaxis_linebreak::BreakError> for BookV2PageRegionLayoutError {
    fn from(e: typaxis_linebreak::BreakError) -> Self {
        Self::Feedback(e)
    }
}

#[derive(Debug)]
pub enum BookV2ObservedPageRegionLayoutError {
    Accounting(typaxis_shaping::book_v2::BookV2ObservedAccountingError),
    Region(BookV2PageRegionLayoutError),
}
impl std::fmt::Display for BookV2ObservedPageRegionLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed region: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedPageRegionLayoutError {}
impl From<BookV2PageRegionLayoutError> for BookV2ObservedPageRegionLayoutError {
    fn from(e: BookV2PageRegionLayoutError) -> Self {
        Self::Region(e)
    }
}
impl From<ProductionInlinePreparationError> for BookV2ObservedPageRegionLayoutError {
    fn from(e: ProductionInlinePreparationError) -> Self {
        Self::Region(e.into())
    }
}
impl From<typaxis_shaping::ProductionTextShapeError> for BookV2ObservedPageRegionLayoutError {
    fn from(e: typaxis_shaping::ProductionTextShapeError) -> Self {
        Self::Region(e.into())
    }
}
impl From<typaxis_linebreak::BreakError> for BookV2ObservedPageRegionLayoutError {
    fn from(e: typaxis_linebreak::BreakError) -> Self {
        Self::Region(e.into())
    }
}
impl From<typaxis_shaping::book_v2::BookV2ObservedPageRegionShapeError>
    for BookV2ObservedPageRegionLayoutError
{
    fn from(e: typaxis_shaping::book_v2::BookV2ObservedPageRegionShapeError) -> Self {
        match e {
            typaxis_shaping::book_v2::BookV2ObservedPageRegionShapeError::Accounting(e) => {
                Self::Accounting(e)
            }
            typaxis_shaping::book_v2::BookV2ObservedPageRegionShapeError::Shape(e) => {
                Self::Region(e.into())
            }
        }
    }
}
fn legacy_region_error(e: BookV2ObservedPageRegionLayoutError) -> BookV2PageRegionLayoutError {
    match e {
        BookV2ObservedPageRegionLayoutError::Region(e) => e,
        BookV2ObservedPageRegionLayoutError::Accounting(_) => {
            unreachable!("private Legacy adapter cannot debit")
        }
    }
}
enum RegionAccounting<'loan, 'id, 'cells, 'window> {
    Legacy,
    Observed {
        accounting:
            &'loan mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
        window: &'loan mut typaxis_shaping::book_v2::BookV2ObservedIntrinsicRecords<'id, 'window>,
    },
}
impl RegionAccounting<'_, '_, '_, '_> {
    fn shape<'a>(
        &mut self,
        policy: &typaxis_syntax::book_v2::BookV2ResourcePolicy<'_>,
        flow: &'a BookV2PageRegionTextFlow<'a>,
        admitted: &'a AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        epoch: [u8; 32],
        lines: Option<&[typaxis_shaping::ProductionParagraphLineContext<'_>]>,
        records: &mut u64,
    ) -> Result<BookV2PageRegionTextShape<'a>, BookV2ObservedPageRegionLayoutError> {
        match self {
            Self::Legacy=>typaxis_shaping::book_v2::shape_book_v2_page_region_text_counted(policy,flow,admitted,limits,epoch,lines,records).map_err(|e|BookV2ObservedPageRegionLayoutError::Region(e.into())),
            Self::Observed{accounting,window}=>typaxis_shaping::book_v2::shape_book_v2_page_region_text_with_observed_source_counted(policy,flow,admitted,limits,epoch,lines,accounting,window,records).map_err(Into::into),
        }
    }
    fn contexts(
        &mut self,
        lines: &BookV2PageRegionLines<'_, '_>,
        records: &mut u64,
    ) -> Result<ProductionSelectedLineContexts, BookV2ObservedPageRegionLayoutError> {
        match self {
            Self::Legacy => lines
                .selected_line_contexts_counted(records)
                .map_err(Into::into),
            Self::Observed { accounting, .. } => {
                accounting
                    .debit_work(NodeId::new(0), 1)
                    .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
                let maximum = accounting
                    .maximum_records()
                    .checked_sub(accounting.counters()[0])
                    .and_then(|n| n.checked_add(lines.records))
                    .ok_or_else(|| {
                        error(
                            NodeId::new(0),
                            ProductionInlinePreparationErrorKind::ArithmeticOverflow,
                        )
                    })?
                    .min(lines.prepared.max_fragments);
                line_context::selected_contexts_observed_counted(
                    lines.paragraphs(),
                    lines.records,
                    maximum,
                    lines.fingerprint,
                    records,
                    accounting,
                )
            }
        }
    }
    fn state(
        &mut self,
        owner: NodeId,
        fp: [u8; 32],
    ) -> Result<typaxis_linebreak::LineLayoutStateFingerprint, BookV2ObservedPageRegionLayoutError>
    {
        if let Self::Observed { accounting, .. } = self {
            const N: usize = b"{\"selected_line_layout_fingerprint\":\"".len() + 64 + 2;
            accounting
                .debit(
                    owner,
                    [
                        1,
                        (typaxis_linebreak::LineLayoutStateFingerprint::ALGORITHM_ID.len() + 1 + N)
                            as u64,
                        (N * 3 + 33) as u64,
                    ],
                )
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        self.boundary(line_context::RegionMetadataKind::State, owner)?;
        super::super::reshape::selected_fingerprint_state(fp).map_err(Into::into)
    }
    fn seal(&mut self) {
        if let Self::Observed { window, .. } = self {
            window.seal();
        }
    }
    fn boundary(
        &self,
        kind: line_context::RegionMetadataKind,
        owner: NodeId,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed { accounting, .. } = self {
            line_context::region_metadata_boundary(kind, accounting.counters(), owner)?;
        }
        Ok(())
    }
    fn reserve<T>(
        &mut self,
        owner: NodeId,
        out: &mut Vec<T>,
        additional: usize,
        append: bool,
        kind: line_context::RegionMetadataKind,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if append {
            self.request_append::<T>(owner, out.len(), additional)?;
        } else {
            self.request::<T>(owner, additional)?;
        }
        self.boundary(kind, owner)?;
        out.try_reserve_exact(additional).map_err(|_| {
            error(
                owner,
                ProductionInlinePreparationErrorKind::AllocationFailure,
            )
        })?;
        Ok(())
    }
    fn validate(
        &self,
        flow: &BookV2PageRegionTextFlow<'_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed { accounting, .. } = self {
            accounting
                .require_limit_domain(limits.base())
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
            accounting
                .require_limit_domain(flow.body().body().limits())
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        Ok(())
    }
    fn limit(&self, max: u64) -> Result<u64, BookV2ObservedPageRegionLayoutError> {
        match self {
            Self::Legacy => Ok(max),
            Self::Observed { accounting, window } => accounting
                .intrinsic_limit(window)
                .map(|m| m.min(max))
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting),
        }
    }
    fn keep(&mut self, n: u64) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed { accounting, window } = self {
            accounting
                .retain_intrinsic_output(window, NodeId::new(0), n)
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        Ok(())
    }
    fn step(&mut self, owner: NodeId, n: u64) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed { accounting, .. } = self {
            accounting
                .debit_work(owner, n)
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        Ok(())
    }
    fn work_limit(&self, maximum: u64) -> u64 {
        match self {
            Self::Legacy => maximum,
            Self::Observed { accounting, .. } => maximum.min(accounting.remaining_work()),
        }
    }
    fn request_append<T>(
        &mut self,
        owner: NodeId,
        len: usize,
        additional: usize,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if matches!(self, Self::Observed { .. }) {
            self.step(owner, 1)?;
            let target = len.checked_add(additional).ok_or_else(|| {
                error(
                    owner,
                    ProductionInlinePreparationErrorKind::ArithmeticOverflow,
                )
            })?;
            self.request::<T>(owner, target)?;
        }
        Ok(())
    }
    fn request<T>(
        &mut self,
        owner: NodeId,
        n: usize,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed { accounting, .. } = self {
            accounting
                .debit_work(owner, 1)
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
            let overflow = || {
                error(
                    owner,
                    ProductionInlinePreparationErrorKind::ArithmeticOverflow,
                )
            };
            let bytes = n
                .checked_mul(std::mem::size_of::<T>())
                .and_then(|v| u64::try_from(v).ok())
                .ok_or_else(overflow)?;
            let work = u64::try_from(n)
                .ok()
                .and_then(|v| v.checked_add(1))
                .ok_or_else(overflow)?;
            accounting
                .debit(owner, [0, bytes, work])
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        Ok(())
    }
}

pub struct BookV2PageRegionInlines<'a> {
    flow: &'a BookV2PageRegionTextFlow<'a>,
    shaped: &'a BookV2PageRegionTextShape<'a>,
    paragraphs: Vec<ProductionPreparedInlineParagraph>,
    records: u64,
    max_fragments: u64,
    fingerprint: [u8; 32],
}
impl<'a> BookV2PageRegionInlines<'a> {
    pub fn flow(&self) -> &'a BookV2PageRegionTextFlow<'a> {
        self.flow
    }
    pub fn shaped(&self) -> &'a BookV2PageRegionTextShape<'a> {
        self.shaped
    }
    pub fn paragraphs(&self) -> &[ProductionPreparedInlineParagraph] {
        &self.paragraphs
    }
    /// Includes caller records, retained shape records and inline records.
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        flow: &BookV2PageRegionTextFlow<'_>,
        shaped: &BookV2PageRegionTextShape<'_>,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.flow, flow) || !std::ptr::eq(self.shaped, shaped) {
            return Err(error(
                NodeId::new(self.flow.source().node_id),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_page_region_inlines<'a>(
    flow: &'a BookV2PageRegionTextFlow<'a>,
    shaped: &'a BookV2PageRegionTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    prior_records: u64,
) -> Result<BookV2PageRegionInlines<'a>, BookV2PageRegionLayoutError> {
    prepare_book_v2_page_region_inlines_counted(
        flow,
        shaped,
        admitted,
        limits,
        epoch,
        japanese_mode,
        prior_records,
        &mut 0,
    )
}

/// Preserve caller history and each accepted retained shape/inline reservation.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_page_region_inlines_counted<'a>(
    flow: &'a BookV2PageRegionTextFlow<'a>,
    shaped: &'a BookV2PageRegionTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    prior_records: u64,
    records: &mut u64,
) -> Result<BookV2PageRegionInlines<'a>, BookV2PageRegionLayoutError> {
    prepare_region_inlines_kernel(
        flow,
        shaped,
        admitted,
        limits,
        epoch,
        japanese_mode,
        prior_records,
        records,
        &mut RegionAccounting::Legacy,
    )
    .map_err(legacy_region_error)
}
/// Same actual region kernel; report is local-zero, not global Source/Style R.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_page_region_inlines_with_observed_source_counted<'a, 'id>(
    flow: &'a BookV2PageRegionTextFlow<'a>,
    shaped: &'a BookV2PageRegionTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    mode: JapaneseLineBreakMode,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, '_>,
    window: &mut typaxis_shaping::book_v2::BookV2ObservedIntrinsicRecords<'id, '_>,
    records: &mut u64,
) -> Result<BookV2PageRegionInlines<'a>, BookV2ObservedPageRegionLayoutError> {
    prepare_region_inlines_kernel(
        flow,
        shaped,
        admitted,
        limits,
        epoch,
        mode,
        0,
        records,
        &mut RegionAccounting::Observed { accounting, window },
    )
}
#[allow(clippy::too_many_arguments)]
fn prepare_region_inlines_kernel<'a>(
    flow: &'a BookV2PageRegionTextFlow<'a>,
    shaped: &'a BookV2PageRegionTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    prior_records: u64,
    records: &mut u64,
    accounting: &mut RegionAccounting<'_, '_, '_, '_>,
) -> Result<BookV2PageRegionInlines<'a>, BookV2ObservedPageRegionLayoutError> {
    *records = prior_records;
    shaped.verify(flow, admitted, limits, epoch)?;
    let owner = NodeId::new(flow.source().node_id);
    accounting.validate(flow, limits)?;
    let maximum = accounting.limit(limits.base().get().max_fragments)?;
    publish_region_preflight(accounting, records, |records| {
        retain(records, shaped.output_records(), maximum, owner)?;
        retain(records, 1, maximum, owner)?;
        retain(
            records,
            flow.text_flow().paragraphs().len() as u64,
            maximum,
            owner,
        )
    })?;
    let paragraphs = prepare_paragraphs_with_record_limit(
        InlineFlow::PageRegion(flow),
        shaped.paragraphs(),
        None,
        None,
        limits,
        japanese_mode,
        records,
        maximum,
    );
    let kept = accounting.keep(*records);
    let paragraphs = paragraphs?;
    kept?;
    let mut fingerprint = sha256(BOOK_V2_PAGE_REGION_INLINE_ALGORITHM.as_bytes());
    for digest in [
        flow.fingerprint(),
        shaped.fingerprint(),
        limits.fingerprint(),
    ]
    .into_iter()
    .chain(paragraphs.iter().map(|p| {
        p.items()
            .map_or([0; 32], ProductionInlineParagraph::fingerprint)
    })) {
        fingerprint = combine(fingerprint, digest);
    }
    Ok(BookV2PageRegionInlines {
        flow,
        shaped,
        paragraphs,
        records: *records,
        max_fragments: maximum,
        fingerprint,
    })
}

/// Translation from a selected line's local glyph coordinates to page space.
/// The inline selector has already applied its origin compensation to glyphs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2PageRegionLineOrigin {
    paragraph_index: u32,
    line_index: u32,
    x: Length,
    y: Length,
    height: PositiveLength,
}
impl BookV2PageRegionLineOrigin {
    pub fn paragraph_index(self) -> u32 {
        self.paragraph_index
    }
    pub fn line_index(self) -> u32 {
        self.line_index
    }
    pub fn x(self) -> Length {
        self.x
    }
    pub fn y(self) -> Length {
        self.y
    }
    pub fn height(self) -> PositiveLength {
        self.height
    }
}

pub struct BookV2PageRegionLines<'p, 'a> {
    prepared: &'p BookV2PageRegionInlines<'a>,
    selected: BookV2SelectedPageMaster<'a>,
    frame: Rect,
    projection: selected::LineProjection<'p, 'a>,
    origins: Vec<BookV2PageRegionLineOrigin>,
    height: NonNegativeLength,
    records: u64,
    fingerprint: [u8; 32],
}
impl<'p, 'a> BookV2PageRegionLines<'p, 'a> {
    pub fn prepared(&self) -> &'p BookV2PageRegionInlines<'a> {
        self.prepared
    }
    pub fn selected_master(&self) -> BookV2SelectedPageMaster<'a> {
        self.selected
    }
    pub fn frame(&self) -> Rect {
        self.frame
    }
    pub fn paragraphs(&self) -> &[ProductionInlineParagraphLineLayout<'p, 'a>] {
        &self.projection.paragraphs
    }
    pub fn origins(&self) -> &[BookV2PageRegionLineOrigin] {
        &self.origins
    }
    pub fn content_height(&self) -> NonNegativeLength {
        self.height
    }
    pub fn output_records(&self) -> u64 {
        self.records
    }
    pub fn candidate_steps(&self) -> u64 {
        self.projection.candidate_steps
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        prepared: &BookV2PageRegionInlines<'_>,
        selected: BookV2SelectedPageMaster<'_>,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.prepared, prepared)
            || !std::ptr::eq(self.selected.source(), selected.source())
            || !std::ptr::eq(self.selected.master(), selected.master())
            || self.selected.page_index() != selected.page_index()
        {
            return Err(error(
                NodeId::new(self.prepared.flow.source().node_id),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
    pub fn selected_line_contexts(
        &self,
    ) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
        self.selected_line_contexts_counted(&mut 0)
    }
    pub fn selected_line_contexts_counted(
        &self,
        observed_records: &mut u64,
    ) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
        line_context::selected_contexts_counted(
            self.paragraphs(),
            self.records,
            self.prepared.max_fragments,
            self.fingerprint,
            observed_records,
        )
    }
    fn check_fit(&self) -> Result<(), BookV2PageRegionLayoutError> {
        if self.height.get() > self.frame.height().get() {
            return Err(BookV2PageRegionLayoutError::Overflow {
                owner: NodeId::new(self.prepared.flow.source().node_id),
                required: self.height.get(),
                available: self.frame.height(),
            });
        }
        Ok(())
    }
}

/// Select in the original master rectangle and reject vertical overflow.
/// Final line-context shaping still requires the convergence callback below.
pub fn layout_book_v2_page_region_lines<'p, 'a>(
    prepared: &'p BookV2PageRegionInlines<'a>,
    selected: BookV2SelectedPageMaster<'a>,
    max_candidate_steps: u64,
    prior_records: u64,
) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2PageRegionLayoutError> {
    let lines = measure_region(prepared, selected, max_candidate_steps, prior_records)?;
    lines.check_fit()?;
    Ok(lines)
}

fn measure_region<'p, 'a>(
    prepared: &'p BookV2PageRegionInlines<'a>,
    selected: BookV2SelectedPageMaster<'a>,
    max_candidate_steps: u64,
    prior_records: u64,
) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2PageRegionLayoutError> {
    measure_region_counted(
        prepared,
        selected,
        max_candidate_steps,
        prior_records,
        &mut 0,
        &mut 0,
    )
}
fn measure_region_counted<'p, 'a>(
    prepared: &'p BookV2PageRegionInlines<'a>,
    selected: BookV2SelectedPageMaster<'a>,
    max_candidate_steps: u64,
    prior_records: u64,
    consumed_steps: &mut u64,
    records: &mut u64,
) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2PageRegionLayoutError> {
    measure_region_kernel(
        prepared,
        selected,
        max_candidate_steps,
        prior_records,
        consumed_steps,
        records,
        prepared.max_fragments,
        &mut RegionAccounting::Legacy,
    )
    .map_err(legacy_region_error)
}
#[allow(clippy::too_many_arguments)]
fn measure_region_kernel<'p, 'a>(
    prepared: &'p BookV2PageRegionInlines<'a>,
    selected: BookV2SelectedPageMaster<'a>,
    max_candidate_steps: u64,
    prior_records: u64,
    consumed_steps: &mut u64,
    records: &mut u64,
    effective_record_limit: u64,
    accounting: &mut RegionAccounting<'_, '_, '_, '_>,
) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2ObservedPageRegionLayoutError> {
    *consumed_steps = 0;
    *records = prior_records;
    use ProductionInlinePreparationErrorKind as E;
    let flow = prepared.flow;
    let owner = NodeId::new(flow.source().node_id);
    let region = match flow.kind() {
        BookV2PageRegionKind::Header => selected.advanced().header_content.as_ref(),
        BookV2PageRegionKind::Footer => selected.advanced().footer_content.as_ref(),
    };
    if !std::ptr::eq(flow.body(), selected.source())
        || flow.master_id() != selected.master().master_id
        || !region.is_some_and(|r| std::ptr::eq(flow.source(), r))
    {
        return Err(error(owner, E::ReceiptMismatch).into());
    }
    let geometry = || BookV2PageRegionLayoutError::Geometry { owner };
    let wire = match flow.kind() {
        BookV2PageRegionKind::Header => selected.master().header,
        BookV2PageRegionKind::Footer => selected.master().footer,
    }
    .ok_or_else(geometry)?;
    let len = |n| Length::from_raw(n).ok_or_else(geometry);
    let x = len(wire.x)?;
    let y = len(wire.y)?;
    let width = PositiveLength::new(len(wire.width)?).ok_or_else(geometry)?;
    let height = PositiveLength::new(len(wire.height)?).ok_or_else(geometry)?;
    if x < Length::ZERO
        || y < Length::ZERO
        || add(x, width.get(), owner)? > len(selected.master().width)?
        || add(y, height.get(), owner)? > len(selected.master().height)?
    {
        return Err(geometry().into());
    }
    let frame = Rect::new(x, y, width, height);
    let maximum = effective_record_limit.min(prepared.max_fragments);
    publish_region_preflight(accounting, records, |records| {
        retain(records, prepared.records, maximum, owner)?;
        retain(records, 1, maximum, owner)?;
        retain(records, prepared.paragraphs.len() as u64, maximum, owner)
    })?;
    let mut widths = Vec::new();
    accounting.reserve(
        owner,
        &mut widths,
        prepared.paragraphs.len(),
        false,
        line_context::RegionMetadataKind::Widths,
    )?;
    for p in flow.text_flow().paragraphs() {
        accounting.step(p.owner(), 1)?;
        let style = p.style().block_style();
        widths.push(
            width
                .get()
                .checked_sub(style.start_indent().get())
                .and_then(|w| w.checked_sub(style.end_indent().get()))
                .and_then(PositiveLength::new)
                .ok_or(BookV2PageRegionLayoutError::Geometry { owner: p.owner() })?,
        );
    }
    let projection = selected::project_lines_counted_with_records(
        selected::LineInputs {
            max_fragments: maximum,
            flow: InlineFlow::PageRegion(flow),
            shaped: prepared.shaped.paragraphs(),
            paragraphs: &prepared.paragraphs,
            footnote_markers: &[],
            native_math: None,
            figure_count: 0,
            fingerprint: prepared.fingerprint,
        },
        &widths,
        accounting.work_limit(max_candidate_steps),
        *records,
        BOOK_V2_PAGE_REGION_LINE_ALGORITHM,
        None,
        consumed_steps,
        records,
    );
    let kept = accounting.keep(*records);
    let spent = accounting.step(owner, *consumed_steps);
    let projection = projection?;
    kept?;
    spent?;
    *records = projection.output_records;
    let mut origins = Vec::new();
    let mut cursor = Length::ZERO;
    let mut previous_after = Length::ZERO;
    let mut has_line = false;
    for (index, (p, source)) in projection
        .paragraphs
        .iter()
        .zip(flow.text_flow().paragraphs())
        .enumerate()
    {
        accounting.step(source.owner(), 1)?;
        let style = source.style().block_style();
        // Empty authored paragraphs use the shared selector's blank line box.
        let lines = p
            .selected()
            .ok_or_else(|| error(source.owner(), E::ReceiptMismatch))?;
        retain(records, lines.lines().len() as u64, maximum, source.owner())?;
        accounting.keep(*records)?;
        accounting.reserve(
            source.owner(),
            &mut origins,
            lines.lines().len(),
            true,
            line_context::RegionMetadataKind::Origins,
        )?;
        if has_line {
            cursor = add(
                cursor,
                add(previous_after, style.space_before().get(), source.owner())?,
                source.owner(),
            )?;
        }
        for (line_index, line) in lines.lines().iter().enumerate() {
            accounting.step(source.owner(), 1)?;
            let slack = p
                .inline_size()
                .get()
                .checked_sub(line.required_inline_size().get())
                .filter(|n| *n >= Length::ZERO)
                .ok_or_else(geometry)?;
            let offset = if line.required_inline_size() == NonNegativeLength::ZERO {
                Length::ZERO
            } else {
                match style.text_align() {
                    typaxis_style::MachineTextAlign::Start => Length::ZERO,
                    typaxis_style::MachineTextAlign::End => slack,
                    typaxis_style::MachineTextAlign::Center => len(slack.raw() / 2)?,
                }
            };
            let line_height = line.line().metrics().line_height();
            origins.push(BookV2PageRegionLineOrigin {
                paragraph_index: u32::try_from(index).map_err(|_| error(owner, E::UnitLimit))?,
                line_index: u32::try_from(line_index).map_err(|_| error(owner, E::UnitLimit))?,
                x: add(
                    add(x, style.start_indent().get(), source.owner())?,
                    offset,
                    source.owner(),
                )?,
                y: add(y, cursor, source.owner())?,
                height: line_height,
            });
            cursor = add(cursor, line_height.get(), source.owner())?;
            has_line = true;
        }
        previous_after = style.space_after().get();
    }
    // As in body flow, leading and trailing paragraph spacing is discarded at
    // the region edges; internal adjacent spacing is additive.
    let mut fingerprint = projection.fingerprint;
    accounting.step(owner, 36 * 2 + 3)?;
    let mut bytes = [0; 36];
    bytes[..4].copy_from_slice(&selected.page_index().to_be_bytes());
    for (chunk, n) in bytes[4..]
        .chunks_exact_mut(8)
        .zip([wire.x, wire.y, wire.width, wire.height])
    {
        chunk.copy_from_slice(&n.to_be_bytes());
    }
    fingerprint = combine(fingerprint, sha256(&bytes));
    Ok(BookV2PageRegionLines {
        prepared,
        selected,
        frame,
        projection,
        origins,
        height: NonNegativeLength::new(cursor).ok_or_else(geometry)?,
        records: *records,
        fingerprint,
    })
}

fn add(a: Length, b: Length, owner: NodeId) -> Result<Length, ProductionInlinePreparationError> {
    a.checked_add(b).ok_or_else(|| {
        error(
            owner,
            ProductionInlinePreparationErrorKind::ArithmeticOverflow,
        )
    })
}
// Local retain accepts each prefix separately. Publish its real accepted
// highwater on every Result before any later allocation/error propagation.
fn publish_region_preflight(
    accounting: &mut RegionAccounting<'_, '_, '_, '_>,
    records: &mut u64,
    reserve: impl FnOnce(&mut u64) -> Result<(), ProductionInlinePreparationError>,
) -> Result<(), BookV2ObservedPageRegionLayoutError> {
    let result = reserve(records);
    let kept = accounting.keep(*records);
    result?;
    kept
}
fn retain(
    records: &mut u64,
    count: u64,
    maximum: u64,
    owner: NodeId,
) -> Result<(), ProductionInlinePreparationError> {
    *records = records
        .checked_add(count)
        .filter(|n| *n <= maximum)
        .ok_or_else(|| error(owner, ProductionInlinePreparationErrorKind::UnitLimit))?;
    Ok(())
}
fn combine(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut bytes = [0; 64];
    bytes[..32].copy_from_slice(&a);
    bytes[32..].copy_from_slice(&b);
    sha256(&bytes)
}

#[cfg(test)]
mod observed_region_geometry_request_tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
    #[test]
    fn book_v2_observed_region_preflight_publishes_earlier_prefix_on_later_refusal() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        for maximum in [6, 8, 9] {
            let mut cells = [5, 7, 11];
            let [r, s, w] = &mut cells;
            with_book_v2_observed_source_accounting(r, s, w, &limits, 5 + maximum, 1000, |a| {
                a.with_zero_intrinsic_records(|a,window|{
                a.retain_intrinsic_output(window,NodeId::new(21),5).unwrap();
                let mut records=0;
                let mut accounting=RegionAccounting::Observed{accounting:a,window};
                let result=publish_region_preflight(&mut accounting,&mut records,|r|{
                    retain(r,5,maximum,NodeId::new(21))?;
                    retain(r,1,maximum,NodeId::new(21))?;
                    retain(r,3,maximum,NodeId::new(21))
                });
                let expected=if maximum<9{6}else{9};
                assert_eq!(records,expected);
                if maximum<9{assert!(matches!(result,Err(BookV2ObservedPageRegionLayoutError::Region(BookV2PageRegionLayoutError::Inline(e))) if e.owner==NodeId::new(21)&&e.kind==ProductionInlinePreparationErrorKind::UnitLimit));}else{assert!(result.is_ok());}
            })
            });
            assert_eq!(cells, [5 + if maximum < 9 { 6 } else { 9 }, 7, 11]);
        }
    }
    #[test]
    fn book_v2_observed_region_geometry_requests_are_paid_before_faults() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        for append in [false, true] {
            for fault in [false, true] {
                let mut cells = [5, 7, 11];
                let (outcome, trace) =
                    line_context::with_region_metadata_fault(fault.then_some(0), || {
                        let [r, s, w] = &mut cells;
                        with_book_v2_observed_source_accounting(r, s, w, &limits, 1000, 1000, |a| {
                            a.with_zero_intrinsic_records(|a, window| {
                                let mut accounting = RegionAccounting::Observed {
                                    accounting: a,
                                    window,
                                };
                                if append {
                                    let mut out: Vec<BookV2PageRegionLineOrigin> = Vec::new();
                                    accounting.reserve(
                                        NodeId::new(3),
                                        &mut out,
                                        3,
                                        true,
                                        line_context::RegionMetadataKind::Origins,
                                    )
                                } else {
                                    let mut out: Vec<PositiveLength> = Vec::new();
                                    accounting.reserve(
                                        NodeId::new(3),
                                        &mut out,
                                        3,
                                        false,
                                        line_context::RegionMetadataKind::Widths,
                                    )
                                }
                            })
                        })
                    });
                let bytes = if append {
                    3 * std::mem::size_of::<BookV2PageRegionLineOrigin>()
                } else {
                    3 * std::mem::size_of::<PositiveLength>()
                };
                assert_eq!(cells, [5, 7 + bytes as u64, if append { 17 } else { 16 }]);
                assert_eq!(trace.len(), 1);
                assert_eq!(trace[0].1, cells);
                assert_eq!(outcome.is_ok(), !fault);
                if let Err(e) = outcome {
                    assert!(
                        matches!(e,BookV2ObservedPageRegionLayoutError::Region(BookV2PageRegionLayoutError::Inline(e)) if e.owner==NodeId::new(3)&&e.kind==ProductionInlinePreparationErrorKind::AllocationFailure)
                    );
                }
            }
        }
    }
    #[test]
    fn book_v2_observed_region_state_request_matches_actual_encoder_and_retains_fault() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        const N: usize = b"{\"selected_line_layout_fingerprint\":\"".len() + 64 + 2;
        let bytes =
            (typaxis_linebreak::LineLayoutStateFingerprint::ALGORITHM_ID.len() + 1 + N) as u64;
        let work = (N * 3 + 33) as u64;
        let fp = [29; 32];
        for axis in 0..3 {
            let mut cells = [
                5,
                if axis == 1 {
                    limits.get().max_spool_bytes - bytes + 1
                } else {
                    7
                },
                11,
            ];
            let before = cells;
            let [r, s, w] = &mut cells;
            let result = with_book_v2_observed_source_accounting(
                r,
                s,
                w,
                &limits,
                if axis == 0 { 5 } else { 6 },
                if axis == 2 { 11 + work - 1 } else { 11 + work },
                |a| {
                    a.with_zero_intrinsic_records(|a, window| {
                        RegionAccounting::Observed {
                            accounting: a,
                            window,
                        }
                        .state(NodeId::new(9), fp)
                    })
                },
            );
            assert!(matches!(
                result,
                Err(BookV2ObservedPageRegionLayoutError::Accounting(_))
            ));
            assert_eq!(cells, before);
        }
        for fault in [false, true] {
            let mut cells = [5, 7, 11];
            let (outcome, trace) =
                line_context::with_region_metadata_fault(fault.then_some(0), || {
                    let [r, s, w] = &mut cells;
                    with_book_v2_observed_source_accounting(r, s, w, &limits, 6, 11 + work, |a| {
                        a.with_zero_intrinsic_records(|a, window| {
                            RegionAccounting::Observed {
                                accounting: a,
                                window,
                            }
                            .state(NodeId::new(9), fp)
                        })
                    })
                });
            assert_eq!(cells, [6, 7 + bytes, 11 + work]);
            assert_eq!(trace, [(line_context::RegionMetadataKind::State, cells)]);
            if fault {
                assert!(outcome.is_err());
            } else {
                assert_eq!(
                    outcome.unwrap(),
                    super::super::super::reshape::selected_fingerprint_state(fp).unwrap()
                );
            }
        }
    }
}
