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
    shaped.verify(flow, admitted, limits, epoch)?;
    let owner = NodeId::new(flow.source().node_id);
    let maximum = limits.base().get().max_fragments;
    let mut records = prior_records;
    retain(&mut records, shaped.output_records(), maximum, owner)?;
    retain(&mut records, 1, maximum, owner)?;
    retain(
        &mut records,
        flow.text_flow().paragraphs().len() as u64,
        maximum,
        owner,
    )?;
    let paragraphs = prepare_paragraphs(
        InlineFlow::PageRegion(flow),
        shaped.paragraphs(),
        None,
        None,
        limits,
        japanese_mode,
        &mut records,
    )?;
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
        records,
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
        line_context::selected_contexts(
            self.paragraphs(),
            self.records,
            self.prepared.max_fragments,
            self.fingerprint,
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
        return Err(geometry());
    }
    let frame = Rect::new(x, y, width, height);
    let maximum = prepared.max_fragments;
    let mut records = prior_records;
    retain(&mut records, prepared.records, maximum, owner)?;
    retain(&mut records, 1, maximum, owner)?;
    retain(
        &mut records,
        prepared.paragraphs.len() as u64,
        maximum,
        owner,
    )?;
    let mut widths = Vec::new();
    widths
        .try_reserve_exact(prepared.paragraphs.len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    for p in flow.text_flow().paragraphs() {
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
    let projection = selected::project_lines(
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
        max_candidate_steps,
        records,
        BOOK_V2_PAGE_REGION_LINE_ALGORITHM,
    )?;
    records = projection.output_records;
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
        let style = source.style().block_style();
        // Empty authored paragraphs use the shared selector's blank line box.
        let lines = p
            .selected()
            .ok_or_else(|| error(source.owner(), E::ReceiptMismatch))?;
        retain(
            &mut records,
            lines.lines().len() as u64,
            maximum,
            source.owner(),
        )?;
        origins
            .try_reserve_exact(lines.lines().len())
            .map_err(|_| error(source.owner(), E::AllocationFailure))?;
        if has_line {
            cursor = add(
                cursor,
                add(previous_after, style.space_before().get(), source.owner())?,
                source.owner(),
            )?;
        }
        for (line_index, line) in lines.lines().iter().enumerate() {
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
        records,
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
