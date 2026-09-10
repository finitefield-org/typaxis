//! Intrinsic vector block geometry bound to actual selected body frames.
use super::*;
use crate::book_v2::{BookV2BoundVector, BookV2InlineLineLayout};
use typaxis_shaping::book_v2::BookV2EquationNumberShapes;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
pub const BOOK_V2_VECTOR_BLOCK_ALGORITHM: &str = "typaxis.book-2-vector-blocks/1";
#[derive(Debug)]
pub enum BookV2VectorBlockError {
    ReceiptMismatch(NodeId),
    MissingFrames,
    OutputLimit,
    Geometry(StagingPrecomposedVectorBlockLayoutError),
    AllocationFailure,
}
impl std::fmt::Display for BookV2VectorBlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 vector blocks: {self:?}")
    }
}
impl std::error::Error for BookV2VectorBlockError {}
pub struct BookV2VectorBlock<'a> {
    binding: &'a BookV2BoundVector<'a>,
    source_event_index: usize,
    inner_left: Length,
    inner_width: PositiveLength,
    viewport_left: Length,
    viewport_width: PositiveLength,
    viewport_height: PositiveLength,
    content_height: PositiveLength,
    viewport_top_offset: NonNegativeLength,
    equation_number: Option<StagingPreparedVectorEquationNumber>,
    fingerprint: [u8; 32],
}
impl<'a> BookV2VectorBlock<'a> {
    fn style(&self) -> BlockStyleRef<'_> {
        match self.binding.placement() {
            PrecomposedVectorPlacementInput::VectorFigure(v) => BlockStyleRef::Figure(v.style()),
            PrecomposedVectorPlacementInput::MathVectorBlock(v) => BlockStyleRef::Math(v.style()),
            _ => unreachable!("private block constructor admits only block placements"),
        }
    }
    pub fn page_name(&self) -> Option<&PageName> {
        match self.binding.placement() {
            PrecomposedVectorPlacementInput::VectorFigure(v) => v.style().page_name(),
            PrecomposedVectorPlacementInput::MathVectorBlock(v) => v.style().page_name(),
            _ => unreachable!("private block constructor admits only block placements"),
        }
    }
    pub fn space_before(&self) -> NonNegativeLength {
        self.style().space_before()
    }
    pub fn space_after(&self) -> NonNegativeLength {
        self.style().space_after()
    }
    pub fn keep_with_next(&self) -> bool {
        self.style().keep_with_next()
    }
    pub fn keep_caption(&self) -> bool {
        self.style().keep_caption()
    }
    pub fn baseline(&self) -> Option<NonNegativeLength> {
        match self.binding.placement() {
            PrecomposedVectorPlacementInput::MathVectorBlock(v) => Some(v.metrics().baseline()),
            _ => None,
        }
    }
    pub fn binding(&self) -> &'a BookV2BoundVector<'a> {
        self.binding
    }
    pub fn owner(&self) -> NodeId {
        self.binding.node_id()
    }
    pub fn source_event_index(&self) -> usize {
        self.source_event_index
    }
    pub fn inner_frame_left(&self) -> Length {
        self.inner_left
    }
    pub fn inner_frame_width(&self) -> PositiveLength {
        self.inner_width
    }
    pub fn viewport_left(&self) -> Length {
        self.viewport_left
    }
    pub fn viewport_width(&self) -> PositiveLength {
        self.viewport_width
    }
    pub fn viewport_height(&self) -> PositiveLength {
        self.viewport_height
    }
    pub fn content_height(&self) -> PositiveLength {
        self.content_height
    }
    pub fn viewport_top_offset(&self) -> NonNegativeLength {
        self.viewport_top_offset
    }
    pub fn equation_number(&self) -> Option<&StagingPreparedVectorEquationNumber> {
        self.equation_number.as_ref()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
/// Carries actual source events and retained number shapes; page assignment is later.
pub struct BookV2VectorBlockLayout<'s, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    numbers: Option<&'s BookV2EquationNumberShapes<'a>>,
    blocks: Vec<BookV2VectorBlock<'a>>,
    record_charge: u64,
    prior_records: u64,
    fingerprint: [u8; 32],
}
impl<'s, 'p, 'a> BookV2VectorBlockLayout<'s, 'p, 'a> {
    pub fn blocks(&self) -> &[BookV2VectorBlock<'a>] {
        &self.blocks
    }
    pub fn numbers(&self) -> Option<&'s BookV2EquationNumberShapes<'a>> {
        self.numbers
    }
    /// Records shared with earlier stages, excluding this set and its numbers.
    pub fn prior_records(&self) -> u64 {
        self.prior_records
    }
    pub fn retained_records(&self) -> u64 {
        self.record_charge - self.prior_records
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        lines: &BookV2InlineLineLayout<'_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2VectorBlockError> {
        if !std::ptr::eq(self.lines, lines)
            || lines.prepared().effective_limits_fingerprint() != limits.fingerprint()
        {
            return Err(BookV2VectorBlockError::ReceiptMismatch(NodeId::new(0)));
        }
        Ok(())
    }
}
pub fn prepare_book_v2_vector_blocks<'s, 'p, 'a>(
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    numbers: Option<&'s BookV2EquationNumberShapes<'a>>,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
) -> Result<Option<BookV2VectorBlockLayout<'s, 'p, 'a>>, BookV2VectorBlockError> {
    use BookV2VectorBlockError as E;
    let prepared = lines.prepared();
    if prepared.effective_limits_fingerprint() != limits.fingerprint() {
        return Err(E::ReceiptMismatch(NodeId::new(0)));
    }
    if let Some(numbers) = numbers {
        numbers
            .verify(prepared.shaped(), limits)
            .map_err(|e| E::ReceiptMismatch(e.owner))?;
    }
    let flow = prepared.source_flow();
    let count = flow
        .events()
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::Begin {
                    kind: Region::VectorFigure | Region::MathVectorBlock,
                    ..
                }
            )
        })
        .count();
    if count == 0 {
        if numbers.is_some() {
            return Err(E::ReceiptMismatch(NodeId::new(0)));
        }
        return Ok(None);
    }
    let frames = lines.frames().ok_or(E::MissingFrames)?;
    let bindings = prepared
        .vector_bindings()
        .ok_or(E::ReceiptMismatch(NodeId::new(0)))?;
    let base = prior_records
        .max(lines.output_records())
        .max(numbers.map_or(0, |n| n.prior_records()))
        .checked_add(numbers.map_or(0, |n| n.retained_records()))
        .ok_or(E::OutputLimit)?;
    let record_charge = base
        .checked_add(1)
        .and_then(|n| n.checked_add(count as u64))
        .and_then(|n| n.checked_add(numbers.map_or(0, |n| n.shapes().len()) as u64))
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or(E::OutputLimit)?;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(count)
        .map_err(|_| E::AllocationFailure)?;
    let mut fingerprint = sha256(BOOK_V2_VECTOR_BLOCK_ALGORITHM.as_bytes());
    let mut number_count = 0usize;
    for (source_event_index, event) in flow.events().iter().enumerate() {
        let Event::Begin {
            owner,
            kind: Region::VectorFigure | Region::MathVectorBlock,
        } = *event
        else {
            continue;
        };
        let binding = bindings.receipt(owner).ok_or(E::ReceiptMismatch(owner))?;
        let (style, width, height) = match binding.placement() {
            PrecomposedVectorPlacementInput::VectorFigure(v) => (
                BlockStyleRef::Figure(v.style()),
                v.viewport_width(),
                v.viewport_height(),
            ),
            PrecomposedVectorPlacementInput::MathVectorBlock(v) => (
                BlockStyleRef::Math(v.style()),
                v.metrics().viewport_width(),
                v.metrics().viewport_height(),
            ),
            _ => return Err(E::ReceiptMismatch(owner)),
        };
        let parent = frames.region(owner).ok_or(E::ReceiptMismatch(owner))?;
        let body = frames.body();
        let x = body.x().checked_add(parent.start()).ok_or(E::Geometry(
            StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow,
        ))?;
        let parent = typaxis_core::Rect::new(x, body.y(), parent.width(), body.height());
        let (inner_left, inner_width, viewport_left) = horizontal_frame(
            parent,
            style.start_indent(),
            style.end_indent(),
            style.text_align(),
            width,
            owner,
            binding.owner_source_span(),
        )
        .map_err(E::Geometry)?;
        let (content_height, viewport_top_offset, equation_number) =
            if let Some(source_number) = binding.source().equation_number() {
                let shape = numbers
                    .and_then(|n| n.shape(owner))
                    .ok_or(E::ReceiptMismatch(owner))?;
                if !std::ptr::eq(shape.source(), binding.source()) {
                    return Err(E::ReceiptMismatch(owner));
                }
                number_count += 1;
                equation_number_geometry(
                    owner,
                    binding.owner_source_span(),
                    source_number,
                    shape.fingerprint(),
                    shape.width(),
                    shape.height(),
                    inner_left,
                    inner_width,
                    viewport_left,
                    width,
                    height,
                )
                .map_err(E::Geometry)?
            } else {
                (height, NonNegativeLength::ZERO, None)
            };
        let mut digest = [0u8; 192];
        digest[..32].copy_from_slice(&fingerprint);
        digest[32..64].copy_from_slice(&lines.fingerprint());
        digest[64..96].copy_from_slice(&binding.fingerprint());
        if let Some(number) = &equation_number {
            digest[96..128].copy_from_slice(&number.shape_fingerprint());
        }
        for (i, value) in [
            inner_left,
            inner_width.get(),
            viewport_left,
            width.get(),
            height.get(),
            content_height.get(),
            viewport_top_offset.get(),
        ]
        .iter()
        .enumerate()
        {
            digest[128 + i * 8..136 + i * 8].copy_from_slice(&value.raw().to_be_bytes());
        }
        let block_fingerprint = sha256(&digest);
        fingerprint = block_fingerprint;
        blocks.push(BookV2VectorBlock {
            binding,
            source_event_index,
            inner_left,
            inner_width,
            viewport_left,
            viewport_width: width,
            viewport_height: height,
            content_height,
            viewport_top_offset,
            equation_number,
            fingerprint: block_fingerprint,
        });
    }
    if blocks.len() != count || number_count != numbers.map_or(0, |n| n.shapes().len()) {
        return Err(E::ReceiptMismatch(NodeId::new(0)));
    }
    Ok(Some(BookV2VectorBlockLayout {
        lines,
        numbers,
        blocks,
        record_charge,
        prior_records: base - numbers.map_or(0, |n| n.retained_records()),
        fingerprint,
    }))
}
