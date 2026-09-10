//! Actual successor formula paints. Tagged structure and publication are later
//! consumers; header copies retain their source terminal's explicit role.
use super::*;
use typaxis_pagination::book_v2::{
    BookV2BodyMathSource, BookV2BodyMathTerminal, BookV2BodyMathTerminals,
};
use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;

#[path = "book_v2_text_display.rs"]
mod text_display;
pub use text_display::*;
#[path = "book_v2_number_display.rs"]
mod number_display;
pub use number_display::*;
#[path = "book_v2_marker_display.rs"]
mod marker_display;
pub use marker_display::*;
#[path = "book_v2_image_display.rs"]
mod image_display;
pub use image_display::*;
#[path = "book_v2_anchor_display.rs"]
mod anchor_display;
pub use anchor_display::*;
#[path = "book_v2_body_display.rs"]
mod body_display;
pub use body_display::*;
#[path = "book_v2_font_uses.rs"]
mod font_uses;
pub use font_uses::*;
#[path = "book_v2_image_uses.rs"]
mod image_uses;
pub use image_uses::*;

pub const BOOK_V2_MATH_DISPLAY_ALGORITHM: &str = "typaxis.book-2-math-display/1";

#[derive(Debug)]
pub enum BookV2MathDisplayError {
    Display(ProductionBodyDisplayError),
    WorkLimit(NodeId),
}
impl From<ProductionBodyDisplayError> for BookV2MathDisplayError {
    fn from(value: ProductionBodyDisplayError) -> Self {
        Self::Display(value)
    }
}
impl std::fmt::Display for BookV2MathDisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 math display: {self:?}")
    }
}
impl std::error::Error for BookV2MathDisplayError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2VectorMathPaint {
    content_key: VectorContentKey,
    viewport: Rect,
    scale_raw: i32,
    matrix: AffineTransform,
    color: [u8; 3],
}
impl BookV2VectorMathPaint {
    pub fn content_key(&self) -> VectorContentKey {
        self.content_key
    }
    pub fn viewport(&self) -> Rect {
        self.viewport
    }
    pub fn scale_raw(&self) -> i32 {
        self.scale_raw
    }
    pub fn matrix(&self) -> AffineTransform {
        self.matrix
    }
    pub fn color(&self) -> [u8; 3] {
        self.color
    }
}
#[derive(Debug)]
pub struct BookV2NativeMathPaint {
    bounds: Rect,
    paints: Vec<ProductionNativeMathPaint>,
}
impl BookV2NativeMathPaint {
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn paints(&self) -> &[ProductionNativeMathPaint] {
        &self.paints
    }
}
#[derive(Debug)]
pub enum BookV2MathPaint {
    Vector(BookV2VectorMathPaint),
    Native(BookV2NativeMathPaint),
}
pub struct BookV2MathDraw<'d, 'r, 'a> {
    terminal: &'d BookV2BodyMathTerminal<'r, 'a>,
    paint: BookV2MathPaint,
}
impl<'d, 'r, 'a> BookV2MathDraw<'d, 'r, 'a> {
    pub fn terminal(&self) -> &'d BookV2BodyMathTerminal<'r, 'a> {
        self.terminal
    }
    /// Original per-occurrence semantics, independent of shared image data.
    pub fn alternative(&self) -> &str {
        match self.terminal.source() {
            BookV2BodyMathSource::Native(n) => &n.source().domain().speech,
            BookV2BodyMathSource::Vector(v) => v.source().alternative().alternative(),
        }
    }
    pub fn actual_text(&self) -> Option<&str> {
        match self.terminal.source() {
            BookV2BodyMathSource::Native(n) => Some(&n.source().domain().speech),
            BookV2BodyMathSource::Vector(v) => v.source().alternative().resolved_actual_text(),
        }
    }
    pub fn paint(&self) -> &BookV2MathPaint {
        &self.paint
    }
}
pub struct BookV2MathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    draws: Vec<BookV2MathDraw<'d, 's, 'p>>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.admitted
    }
    pub fn draws(&self) -> &[BookV2MathDraw<'d, 's, 'p>] {
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

/// A single cumulative builder retains charges across successful, abandoned or
/// failed projections. Prior counters account for other intervening stages.
pub struct BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    maximum_records: u64,
    remaining: u64,
    maximum_work: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn new(
        source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        admitted: &'d AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_work: u64,
    ) -> Result<Self, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let flow = source.source().flow();
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)
            .map_err(|_| error(root, E::ReceiptMismatch))?;
        let prepared = flow.lines().prepared();
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
        let mut remaining = limits
            .base()
            .get()
            .max_fragments
            .checked_sub(prior_records.max(source.record_charge()))
            .ok_or_else(|| error(root, E::RecordLimit))?;
        let work = prior_work.max(source.work_steps());
        if work > maximum_work {
            return Err(BookV2MathDisplayError::WorkLimit(root));
        }
        take(&mut remaining, 1, root)?;
        let mut result = Self {
            source,
            admitted,
            remaining,
            maximum_records: limits.base().get().max_fragments,
            maximum_work,
            work,
        };
        if source.source().has_header_variants() {
            for page in source.source().geometry().pages() {
                result.step(root)?;
                for variant in page.header_variants() {
                    result.step(root)?;
                    let flow = variant.measurements().flow();
                    flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)
                        .map_err(|_| error(root, E::ReceiptMismatch))?;
                    let prepared = flow.lines().prepared();
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
                }
            }
        }
        Ok(result)
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
    fn fold(
        &mut self,
        previous: [u8; 32],
        value: &[u8],
        owner: NodeId,
    ) -> Result<[u8; 32], BookV2MathDisplayError> {
        for _ in 0..(32 + value.len()).div_ceil(64) {
            self.step(owner)?;
        }
        Ok(fold(previous, value))
    }
    pub fn build(
        &mut self,
    ) -> Result<BookV2MathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let source = self.source;
        let mut required = 1usize;
        for terminal in source.terminals() {
            self.step(terminal.source().owner())?;
            let paints = match terminal.source() {
                BookV2BodyMathSource::Native(n) => n.computation().paints().len(),
                _ => 0,
            };
            required = required
                .checked_add(1)
                .and_then(|n| n.checked_add(paints))
                .ok_or_else(|| error(root, E::RecordLimit))?;
        }
        if required as u64 > self.remaining {
            return Err(error(root, E::RecordLimit).into());
        }
        // Reserve all retained draw/paint slots before allocating the array.
        // Failures retain this reservation, including paints not yet reached.
        take(&mut self.remaining, required, root)?;
        let mut draws = Vec::new();
        draws
            .try_reserve_exact(source.terminals().len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        self.step(root)?;
        let mut fingerprint = sha256(BOOK_V2_MATH_DISPLAY_ALGORITHM.as_bytes());
        for hash in [source.fingerprint(), self.admitted.fingerprint()] {
            fingerprint = self.fold(fingerprint, &hash, root)?;
        }
        for terminal in source.terminals() {
            let owner = terminal.source().owner();
            self.step(owner)?;
            let paint = match terminal.source() {
                BookV2BodyMathSource::Vector(binding) => {
                    let image = self
                        .admitted
                        .image(binding.resource().image_id())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let content_key = crate::precomposed_vector::content_key_for_resource(
                        binding.resource(),
                        image,
                    )
                    .map_err(|_| error(owner, E::ReceiptMismatch))?;
                    let (scale_raw, color) = match binding.placement() {
                        PrecomposedVectorPlacementInput::Inline(p) => {
                            (p.scale().get().raw(), p.paint())
                        }
                        PrecomposedVectorPlacementInput::MathVectorBlock(p) => {
                            (p.scale().get().raw(), p.paint())
                        }
                        _ => return Err(error(owner, E::ReceiptMismatch).into()),
                    };
                    let viewport = terminal
                        .viewport()
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    BookV2MathPaint::Vector(BookV2VectorMathPaint {
                        content_key,
                        viewport,
                        scale_raw,
                        matrix: placement_matrix(viewport, scale_raw),
                        color: [color.red(), color.green(), color.blue()],
                    })
                }
                BookV2BodyMathSource::Native(receipt) => {
                    let face = self
                        .admitted
                        .font(receipt.font_face_id())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if face.content_hash() != receipt.font_sha256()
                        || face.face_index() != receipt.face_index()
                    {
                        return Err(error(owner, E::ReceiptMismatch).into());
                    }
                    for _ in receipt.computation().paints() {
                        self.step(owner)?;
                    }
                    let mut reserved = u64::try_from(receipt.computation().paints().len())
                        .ok()
                        .and_then(|n| n.checked_add(1))
                        .ok_or_else(|| error(owner, E::RecordLimit))?;
                    let (bounds, paints) = native_math::project_native_paints(
                        owner,
                        receipt.computation(),
                        terminal.origin_x(),
                        terminal.baseline(),
                        &mut reserved,
                    )?;
                    BookV2MathPaint::Native(BookV2NativeMathPaint { bounds, paints })
                }
            };
            fingerprint = self.fold(fingerprint, &owner.get().to_be_bytes(), owner)?;
            match &paint {
                BookV2MathPaint::Vector(v) => {
                    let mut bytes = [0u8; 104];
                    bytes[..32].copy_from_slice(&v.content_key.source_sha256());
                    bytes[32..64].copy_from_slice(&v.content_key.ir_fingerprint());
                    put_rect(&mut bytes[64..96], v.viewport);
                    bytes[96..100].copy_from_slice(&v.scale_raw.to_be_bytes());
                    bytes[100..103].copy_from_slice(&v.color);
                    fingerprint = self.fold(fingerprint, &bytes, owner)?;
                }
                BookV2MathPaint::Native(n) => {
                    let mut bytes = [0u8; 33];
                    bytes[32] = 1;
                    put_rect(&mut bytes[..32], n.bounds);
                    fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    for paint in &n.paints {
                        fingerprint = self.fold(fingerprint, &native_paint_bytes(*paint), owner)?;
                    }
                }
            }
            draws.push(BookV2MathDraw { terminal, paint });
        }
        Ok(BookV2MathDisplay {
            source,
            admitted: self.admitted,
            draws,
            fingerprint,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }
}

fn fold(previous: [u8; 32], value: &[u8]) -> [u8; 32] {
    let mut bytes = [0u8; 136];
    bytes[..32].copy_from_slice(&previous);
    bytes[32..32 + value.len()].copy_from_slice(value);
    sha256(&bytes[..32 + value.len()])
}
fn put_rect(bytes: &mut [u8], rect: Rect) {
    for (i, value) in [rect.x(), rect.y(), rect.width().get(), rect.height().get()]
        .into_iter()
        .enumerate()
    {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&value.raw().to_be_bytes());
    }
}
fn native_paint_bytes(paint: ProductionNativeMathPaint) -> [u8; 35] {
    let mut bytes = [0; 35];
    match paint {
        ProductionNativeMathPaint::Rule(rect) => {
            bytes[0] = 1;
            put_rect(&mut bytes[1..33], rect);
        }
        ProductionNativeMathPaint::Glyph {
            original_gid,
            unicode,
            logical_ordinal,
            x,
            y,
            font_size,
        } => {
            bytes[1..3].copy_from_slice(&original_gid.get().to_be_bytes());
            bytes[3..7].copy_from_slice(&(unicode as u32).to_be_bytes());
            bytes[7..11].copy_from_slice(&logical_ordinal.to_be_bytes());
            bytes[11..19].copy_from_slice(&x.raw().to_be_bytes());
            bytes[19..27].copy_from_slice(&y.raw().to_be_bytes());
            bytes[27..35].copy_from_slice(&font_size.get().raw().to_be_bytes());
        }
    }
    bytes
}
