//! Source-ordered page-space projection. Structure/paint permits are separate
//! owners; this stage does not authorize an untagged replacement PDF.
use crate::precomposed_vector::{binding_paint, content_key_for, placement_matrix};
use typaxis_core::{
    sha256, AffineTransform, DisplayTextBufferId, DisplayTextSpan, FontFaceId, Length,
    M4EffectiveResourceLimits, NodeId, PositiveLength, Rect,
};
use typaxis_font::OriginalGlyphId;
use typaxis_layout::PrecomposedVectorPlacementInput;
use typaxis_layout::{
    ProductionPlacedInline, ProductionPlacedInlineAnchor, ValidatedMathVectorReceipt,
    ValidatedPrecomposedVectorReceipt,
};
use typaxis_pagination::{ProductionBodyFragmentSource, ProductionBodySelectedLayout};
use typaxis_resource_admission::{AdmittedResourceLedger, VectorContentKey};
use typaxis_syntax::PrecomposedVectorKind;

#[path = "production_equation_numbers.rs"]
mod equation_numbers;
#[path = "production_list.rs"]
mod list;

pub const PRODUCTION_BODY_DISPLAY_ALGORITHM: &str = "typaxis.production-body-display/5";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyDisplayErrorKind {
    ReceiptMismatch,
    PendingEquationNumber,
    RecordLimit,
    AllocationFailure,
    ArithmeticOverflow,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyDisplayError {
    pub owner: NodeId,
    pub kind: ProductionBodyDisplayErrorKind,
}
impl std::fmt::Display for ProductionBodyDisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "production_body_display {:?}: node {}",
            self.kind,
            self.owner.get()
        )
    }
}
impl std::error::Error for ProductionBodyDisplayError {}
use ProductionBodyDisplayErrorKind as E;
fn error(owner: NodeId, kind: E) -> ProductionBodyDisplayError {
    ProductionBodyDisplayError { owner, kind }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyGlyph {
    original_gid: OriginalGlyphId,
    x: Length,
    y: Length,
}
impl ProductionBodyGlyph {
    pub const fn original_gid(self) -> OriginalGlyphId {
        self.original_gid
    }
    pub const fn x(self) -> Length {
        self.x
    }
    pub const fn y(self) -> Length {
        self.y
    }
}
#[derive(Debug)]
pub struct ProductionBodyTextDraw<'d> {
    owner: NodeId,
    page_index: u32,
    fragment_index: u32,
    font_face_id: FontFaceId,
    font_sha256: [u8; 32],
    face_index: u32,
    font_size: PositiveLength,
    text_span: DisplayTextSpan,
    exact_text: &'d str,
    generated_provenance: Option<typaxis_text::GeneratedProvenance>,
    equation_number: Option<&'d typaxis_shaping::StagingEquationNumberShapeReceipt>,
    logical_bounds: Option<Rect>,
    glyphs: Vec<ProductionBodyGlyph>,
}
impl<'d> ProductionBodyTextDraw<'d> {
    pub fn equation_number(
        &self,
    ) -> Option<&'d typaxis_shaping::StagingEquationNumberShapeReceipt> {
        self.equation_number
    }
    pub const fn generated_provenance(&self) -> Option<typaxis_text::GeneratedProvenance> {
        self.generated_provenance
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn font_sha256(&self) -> [u8; 32] {
        self.font_sha256
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub const fn font_size(&self) -> PositiveLength {
        self.font_size
    }
    pub const fn text_span(&self) -> DisplayTextSpan {
        self.text_span
    }
    pub const fn exact_text(&self) -> &'d str {
        self.exact_text
    }
    pub fn glyphs(&self) -> &[ProductionBodyGlyph] {
        &self.glyphs
    }
    /// Selected pen/advance and actual font ascent/descent in page coordinates.
    /// This is a logical interaction box, not a glyph ink bounding box. A
    /// zero-extent cluster has no positive-area box; no extent is invented.
    pub const fn logical_bounds(&self) -> Option<Rect> {
        self.logical_bounds
    }
}
#[derive(Debug)]
pub struct ProductionBodyVectorDraw<'d> {
    page_index: u32,
    fragment_index: u32,
    binding: &'d ValidatedPrecomposedVectorReceipt,
    math_binding: Option<&'d ValidatedMathVectorReceipt>,
    viewport: Rect,
    baseline: Option<Length>,
    content_key: VectorContentKey,
    scale_raw: i32,
    matrix: AffineTransform,
    color: [u8; 3],
    fingerprint: [u8; 32],
}
impl<'d> ProductionBodyVectorDraw<'d> {
    pub const fn baseline(&self) -> Option<Length> {
        self.baseline
    }
    pub const fn content_key(&self) -> VectorContentKey {
        self.content_key
    }
    pub const fn scale_raw(&self) -> i32 {
        self.scale_raw
    }
    pub const fn matrix(&self) -> AffineTransform {
        self.matrix
    }
    pub const fn resolved_current_color(&self) -> [u8; 3] {
        self.color
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn binding(&self) -> &'d ValidatedPrecomposedVectorReceipt {
        self.binding
    }
    pub const fn math_binding(&self) -> Option<&'d ValidatedMathVectorReceipt> {
        self.math_binding
    }
    pub const fn viewport(&self) -> Rect {
        self.viewport
    }
}
#[derive(Debug)]
pub enum ProductionBodyDraw<'d> {
    Text(ProductionBodyTextDraw<'d>),
    Vector(ProductionBodyVectorDraw<'d>),
    Raster(ProductionBodyRasterDraw<'d>),
}

#[derive(Debug)]
pub struct ProductionBodyRasterDraw<'d> {
    owner: NodeId,
    page_index: u32,
    fragment_index: u32,
    image: &'d typaxis_resource_admission::AdmittedImage,
    alternative: &'d str,
    viewport: Rect,
}
impl<'d> ProductionBodyRasterDraw<'d> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn image(&self) -> &'d typaxis_resource_admission::AdmittedImage {
        self.image
    }
    pub const fn alternative(&self) -> &'d str {
        self.alternative
    }
    pub const fn viewport(&self) -> Rect {
        self.viewport
    }
}

/// A nonpainting source anchor translated by its actual selected page fragment.
/// Kept separate from draws: it cannot create text, MCIDs or duplicate SVG paint.
#[derive(Clone, Copy, Debug)]
pub struct ProductionBodyInlineAnchor<'d> {
    source: &'d ProductionPlacedInlineAnchor,
    page_index: u32,
    fragment_index: u32,
    x: Length,
    baseline: Length,
}
impl<'d> ProductionBodyInlineAnchor<'d> {
    pub const fn source(&self) -> &'d ProductionPlacedInlineAnchor {
        self.source
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn x(&self) -> Length {
        self.x
    }
    pub const fn baseline(&self) -> Length {
        self.baseline
    }
}

pub struct ProductionBodyDisplay<'d, 's, 'p, 'a> {
    selected: &'d ProductionBodySelectedLayout<'s, 'p, 'a>,
    admitted: &'d AdmittedResourceLedger,
    draws: Vec<ProductionBodyDraw<'d>>,
    inline_anchors: Vec<ProductionBodyInlineAnchor<'d>>,
    record_charge: u64,
    fingerprint: [u8; 32],
}
impl<'d, 's, 'p, 'a> ProductionBodyDisplay<'d, 's, 'p, 'a> {
    pub fn resource_declarations(&self) -> &typaxis_document::StagingM4ResourceCatalog {
        self.selected
            .line_layout()
            .source_flow()
            .resource_declarations()
    }
    pub fn draws(&self) -> &[ProductionBodyDraw<'d>] {
        &self.draws
    }
    pub fn inline_anchors(&self) -> &[ProductionBodyInlineAnchor<'d>] {
        &self.inline_anchors
    }
    pub const fn selected(&self) -> &'d ProductionBodySelectedLayout<'s, 'p, 'a> {
        self.selected
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify_resources(
        &self,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyDisplayError> {
        if !std::ptr::eq(self.admitted, admitted)
            || self
                .selected
                .line_layout()
                .binding_epoch()
                .limits_fingerprint()
                != limits.fingerprint()
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    pub fn verify(
        &self,
        selected: &ProductionBodySelectedLayout<'_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyDisplayError> {
        self.verify_resources(admitted, limits)?;
        if !std::ptr::eq(self.selected, selected) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

pub fn build_production_body_display<'d, 's, 'p, 'a>(
    selected: &'d ProductionBodySelectedLayout<'s, 'p, 'a>,
    admitted: &'d AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyDisplay<'d, 's, 'p, 'a>, ProductionBodyDisplayError> {
    let root = NodeId::new(0);
    let lines = selected.line_layout();
    if lines.binding_epoch().admitted_fingerprint() != admitted.fingerprint().bytes()
        || lines.binding_epoch().limits_fingerprint() != limits.fingerprint()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut remaining = limits
        .base()
        .get()
        .max_fragments
        .checked_sub(selected.record_charge())
        .ok_or_else(|| error(root, E::RecordLimit))?;
    let mut draws = Vec::new();
    let mut inline_anchors = Vec::new();
    let mut marker_cursor = 0;
    for (fragment_index, fragment) in selected.fragments().iter().enumerate() {
        let index =
            u32::try_from(fragment_index).map_err(|_| error(fragment.owner(), E::RecordLimit))?;
        while let Some(marker) = selected.list_markers().get(marker_cursor) {
            if marker.fragment_index() != index {
                break;
            }
            list::append_marker(selected, marker, admitted, &mut remaining, &mut draws)?;
            marker_cursor += 1;
        }
        match fragment.source() {
            ProductionBodyFragmentSource::RasterFigure { figure_index } => {
                let figure = lines
                    .figures()
                    .get(figure_index as usize)
                    .filter(|f| f.owner() == fragment.owner())
                    .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?;
                let image = admitted
                    .image(figure.image_id())
                    .ok_or_else(|| error(figure.owner(), E::ReceiptMismatch))?;
                if image.content_hash() != figure.admitted_sha256()
                    || image.width().get() != figure.pixel_width()
                    || image.height().get() != figure.pixel_height()
                {
                    return Err(error(figure.owner(), E::ReceiptMismatch));
                }
                take(&mut remaining, 1, figure.owner())?;
                draws
                    .try_reserve(1)
                    .map_err(|_| error(figure.owner(), E::AllocationFailure))?;
                draws.push(ProductionBodyDraw::Raster(ProductionBodyRasterDraw {
                    owner: figure.owner(),
                    page_index: fragment.page_index(),
                    fragment_index: index,
                    image,
                    alternative: figure.source().alternative(),
                    viewport: fragment
                        .viewport()
                        .ok_or_else(|| error(figure.owner(), E::ReceiptMismatch))?,
                }));
            }
            ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } => {
                let p = &lines.paragraphs()[paragraph_index as usize];
                // Source gap order implies nondecreasing selected line order.
                // Binary range lookup avoids rescanning all paragraph anchors
                // for each line of a long chapter.
                let first = p
                    .anchors()
                    .partition_point(|a| a.position().is_some_and(|p| p.line_index() < line_index));
                for anchor in &p.anchors()[first..] {
                    let position = anchor
                        .position()
                        .ok_or_else(|| error(anchor.source().owner(), E::ReceiptMismatch))?;
                    if position.line_index() != line_index {
                        break;
                    }
                    let owner = anchor.source().owner();
                    take(&mut remaining, 1, owner)?;
                    inline_anchors
                        .try_reserve(1)
                        .map_err(|_| error(owner, E::AllocationFailure))?;
                    let baseline = plus(fragment.bounds().y(), position.baseline(), owner)?;
                    if fragment.baseline() != Some(baseline) {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    inline_anchors.push(ProductionBodyInlineAnchor {
                        source: anchor,
                        page_index: fragment.page_index(),
                        fragment_index: index,
                        x: plus(fragment.bounds().x(), position.x(), owner)?,
                        baseline,
                    });
                }
                for item in p.lines()[line_index as usize].items() {
                    let draw = match item {
                        ProductionPlacedInline::Text(cluster) => {
                            let owner = cluster.run().owner();
                            let font = p.font().ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            let face = admitted
                                .font(font.face_id())
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            if face.content_hash() != font.content_hash()
                                || face.face_index() != font.face_index()
                            {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                            take(&mut remaining, cluster.glyphs().len() + 1, owner)?;
                            let mut glyphs = Vec::new();
                            let mut advance = Length::ZERO;
                            glyphs
                                .try_reserve_exact(cluster.glyphs().len())
                                .map_err(|_| error(owner, E::AllocationFailure))?;
                            for glyph in cluster.glyphs() {
                                advance = plus(advance, glyph.glyph().advance_x, owner)?;
                                glyphs.push(ProductionBodyGlyph {
                                    original_gid: glyph.glyph().original_gid,
                                    x: plus(fragment.bounds().x(), glyph.x(), owner)?,
                                    y: plus(fragment.bounds().y(), glyph.y(), owner)?,
                                });
                            }
                            let height = font
                                .ascender()
                                .checked_sub(font.descender())
                                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                            if advance.raw() < 0 || height.raw() < 0 {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                            let logical_bounds = if let (Some(width), Some(height)) =
                                (PositiveLength::new(advance), PositiveLength::new(height))
                            {
                                let baseline = fragment
                                    .baseline()
                                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                                Some(Rect::new(
                                    plus(fragment.bounds().x(), cluster.pen_x(), owner)?,
                                    baseline
                                        .checked_sub(font.ascender())
                                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                                    width,
                                    height,
                                ))
                            } else {
                                None
                            };
                            let span = cluster.source_span();
                            ProductionBodyDraw::Text(ProductionBodyTextDraw {
                                owner,
                                page_index: fragment.page_index(),
                                fragment_index: index,
                                font_face_id: font.face_id(),
                                font_sha256: font.content_hash(),
                                face_index: font.face_index(),
                                font_size: font.size(),
                                text_span: DisplayTextSpan::new(
                                    DisplayTextBufferId::new(span.text_id().get()),
                                    span.start_byte(),
                                    span.end_byte(),
                                )
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                                exact_text: cluster.utf8(),
                                generated_provenance: None,
                                equation_number: None,
                                logical_bounds,
                                glyphs,
                            })
                        }
                        ProductionPlacedInline::Vector(vector) => {
                            let local = vector.geometry().viewport();
                            let owner = vector.occurrence().item().node_id();
                            take(&mut remaining, 1, owner)?;
                            ProductionBodyDraw::Vector(vector_draw(
                                selected,
                                admitted,
                                owner,
                                index,
                                fragment.page_index(),
                                Rect::new(
                                    plus(fragment.bounds().x(), local.x(), owner)?,
                                    plus(fragment.bounds().y(), local.y(), owner)?,
                                    local.width(),
                                    local.height(),
                                ),
                            )?)
                        }
                        ProductionPlacedInline::Break(_) => continue, // No paint/text; the selected flow retains the node.
                    };
                    draws
                        .try_reserve(1)
                        .map_err(|_| error(fragment.owner(), E::AllocationFailure))?;
                    draws.push(draw);
                }
            }
            ProductionBodyFragmentSource::VectorBlock { block_index } => {
                let block = &selected.block_layout().blocks()[block_index as usize];
                take(&mut remaining, 1, block.owner())?;
                draws
                    .try_reserve(1)
                    .map_err(|_| error(block.owner(), E::AllocationFailure))?;
                draws.push(ProductionBodyDraw::Vector(vector_draw(
                    selected,
                    admitted,
                    block.owner(),
                    index,
                    fragment.page_index(),
                    fragment
                        .viewport()
                        .ok_or_else(|| error(block.owner(), E::ReceiptMismatch))?,
                )?));
                if block.equation_number().is_some() {
                    equation_numbers::append_number(
                        selected,
                        block.owner(),
                        index,
                        admitted,
                        &mut remaining,
                        &mut draws,
                    )?;
                }
            }
        }
    }
    if marker_cursor != selected.list_markers().len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut digest = [0u8; 64];
    digest[..32].copy_from_slice(&sha256(PRODUCTION_BODY_DISPLAY_ALGORITHM.as_bytes()));
    digest[32..].copy_from_slice(&selected.fingerprint());
    Ok(ProductionBodyDisplay {
        selected,
        admitted,
        draws,
        inline_anchors,
        record_charge: limits.base().get().max_fragments - remaining,
        fingerprint: sha256(&digest),
    })
}
fn vector_draw<'d>(
    selected: &'d ProductionBodySelectedLayout<'_, '_, '_>,
    admitted: &AdmittedResourceLedger,
    owner: NodeId,
    fragment_index: u32,
    page_index: u32,
    viewport: Rect,
) -> Result<ProductionBodyVectorDraw<'d>, ProductionBodyDisplayError> {
    let binding = selected
        .line_layout()
        .vector_binding(owner)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let math_binding = selected.line_layout().math_vector_binding(owner);
    let is_math = matches!(
        binding.kind(),
        PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock
    );
    if is_math != math_binding.is_some()
        || math_binding.is_some_and(|m| m.common_fingerprint() != binding.fingerprint())
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let content_key =
        content_key_for(binding, admitted).map_err(|_| error(owner, E::ReceiptMismatch))?;
    let scale_raw = match binding.placement() {
        PrecomposedVectorPlacementInput::Inline(p) => p.scale().get().raw(),
        PrecomposedVectorPlacementInput::VectorFigure(p) => p.scale().get().raw(),
        PrecomposedVectorPlacementInput::MathVectorBlock(p) => p.scale().get().raw(),
    };
    let paint = binding_paint(binding);
    let mut fingerprint_input = [0u8; 72];
    fingerprint_input[..32].copy_from_slice(&selected.fingerprint());
    fingerprint_input[32..64].copy_from_slice(&binding.fingerprint());
    fingerprint_input[64..68].copy_from_slice(&fragment_index.to_be_bytes());
    fingerprint_input[68..].copy_from_slice(&page_index.to_be_bytes());
    Ok(ProductionBodyVectorDraw {
        baseline: selected
            .fragments()
            .get(fragment_index as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
            .baseline(),
        content_key,
        scale_raw,
        matrix: placement_matrix(viewport, scale_raw),
        color: [paint.red(), paint.green(), paint.blue()],
        fingerprint: sha256(&fingerprint_input),
        page_index,
        fragment_index,
        binding,
        math_binding,
        viewport,
    })
}
fn plus(a: Length, b: Length, owner: NodeId) -> Result<Length, ProductionBodyDisplayError> {
    a.checked_add(b)
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))
}
fn take(remaining: &mut u64, n: usize, owner: NodeId) -> Result<(), ProductionBodyDisplayError> {
    *remaining = remaining
        .checked_sub(n as u64)
        .ok_or_else(|| error(owner, E::RecordLimit))?;
    Ok(())
}
