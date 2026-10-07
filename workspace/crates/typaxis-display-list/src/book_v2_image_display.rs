//! Original ordinary images and non-formula vectors in actual page space.
use super::*;
use typaxis_layout::book_v2::BookV2BoundVector;
use typaxis_layout::{ProductionFigureMedia, ProductionPreparedFigure};
use typaxis_pagination::{ProductionBodyFootnotePlacedFragment, ProductionTablePlacedCellRole};
use typaxis_resource_admission::AdmittedImage;

pub const BOOK_V2_IMAGE_DISPLAY_ALGORITHM: &str = "typaxis.book-2-image-display/1";

#[derive(Clone, Copy)]
pub enum BookV2ImageSource<'p, 'a> {
    Figure(&'p ProductionPreparedFigure<'a>),
    Vector(&'p BookV2BoundVector<'a>),
}
impl BookV2ImageSource<'_, '_> {
    pub fn owner(&self) -> NodeId {
        match self {
            Self::Figure(f) => f.owner(),
            Self::Vector(v) => v.node_id(),
        }
    }
    pub fn alternative(&self) -> &str {
        match self {
            Self::Figure(f) => f.source().alternative(),
            Self::Vector(v) => v.source().alternative().alternative(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ImagePaint {
    Raster {
        viewport: Rect,
    },
    Vector {
        content_key: VectorContentKey,
        viewport: Rect,
        scale_raw: i32,
        matrix: AffineTransform,
        color: [u8; 3],
    },
}
impl BookV2ImagePaint {
    pub fn viewport(&self) -> Rect {
        match self {
            Self::Raster { viewport } | Self::Vector { viewport, .. } => *viewport,
        }
    }
}
pub struct BookV2ImageDraw<'d, 'g, 'p, 'a> {
    source: BookV2ImageSource<'p, 'a>,
    image: &'d AdmittedImage,
    fragment: &'g ProductionBodyFootnotePlacedFragment,
    index: usize,
    inline: Option<u32>,
    cell: Option<ProductionTablePlacedCellRole>,
    repeated: bool,
    paint: BookV2ImagePaint,
}
impl<'d, 'g, 'p, 'a> BookV2ImageDraw<'d, 'g, 'p, 'a> {
    pub fn source(&self) -> BookV2ImageSource<'p, 'a> {
        self.source
    }
    pub fn image(&self) -> &'d AdmittedImage {
        self.image
    }
    pub fn fragment(&self) -> &'g ProductionBodyFootnotePlacedFragment {
        self.fragment
    }
    pub fn fragment_index(&self) -> usize {
        self.index
    }
    pub fn inline_index(&self) -> Option<u32> {
        self.inline
    }
    pub fn cell_role(&self) -> Option<ProductionTablePlacedCellRole> {
        self.cell
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
    pub fn paint(&self) -> BookV2ImagePaint {
        self.paint
    }
}
pub struct BookV2ImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    draws: Vec<BookV2ImageDraw<'d, 'g, 'p, 'a>>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.admitted
    }
    pub fn draws(&self) -> &[BookV2ImageDraw<'d, 'g, 'p, 'a>] {
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

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    /// Both passes visit immutable actual placements. The first allocates
    /// nothing; all retained draw slots are charged before the second starts.
    fn visit_images(
        &mut self,
        mut emit: impl FnMut(
            &mut Self,
            BookV2ImageDraw<'d, 'g, 'p, 'a>,
        ) -> Result<(), BookV2MathDisplayError>,
    ) -> Result<(), BookV2MathDisplayError> {
        let mut index = 0usize;
        for page in self.source.source().geometry().pages() {
            self.step(NodeId::new(0))?;
            for (placed, role, repeated) in page.fragments_with_roles() {
                let fragment = placed.fragment();
                let owner = fragment.owner();
                self.step(owner)?;
                let flow = self
                    .source
                    .fragment_flow(index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let prepared = flow.lines().prepared();
                let mut push = |this: &mut Self, source, inline, viewport| {
                    let (image, paint) = this.image_paint(source, viewport)?;
                    emit(
                        this,
                        BookV2ImageDraw {
                            source,
                            image,
                            fragment: placed,
                            index,
                            inline,
                            cell: role,
                            repeated,
                            paint,
                        },
                    )
                };
                match fragment.source() {
                    ProductionBodyFragmentSource::Figure { figure_index } => {
                        let figure = prepared
                            .figures()
                            .get(figure_index as usize)
                            .filter(|f| f.owner() == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let viewport = fragment
                            .viewport()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        push(self, BookV2ImageSource::Figure(figure), None, viewport)?;
                    }
                    ProductionBodyFragmentSource::VectorBlock { block_index } => {
                        let block = flow
                            .blocks()
                            .and_then(|b| b.blocks().get(block_index as usize))
                            .filter(|b| b.owner() == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        match block.binding().kind() {
                            PrecomposedVectorKind::VectorFigure => {
                                let viewport = fragment
                                    .viewport()
                                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                                push(
                                    self,
                                    BookV2ImageSource::Vector(block.binding()),
                                    None,
                                    viewport,
                                )?;
                            }
                            PrecomposedVectorKind::MathVectorBlock => (),
                            _ => return Err(error(owner, E::ReceiptMismatch).into()),
                        }
                    }
                    ProductionBodyFragmentSource::ParagraphLine {
                        paragraph_index,
                        line_index,
                    } => {
                        let paragraph = flow
                            .lines()
                            .paragraphs()
                            .get(paragraph_index as usize)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let line = paragraph
                            .lines()
                            .get(line_index as usize)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        for (inline, item) in line.items().iter().enumerate() {
                            self.step(owner)?;
                            let ProductionPlacedInline::Vector(selected) = item else {
                                continue;
                            };
                            let bindings = prepared
                                .vector_bindings()
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            for _ in 0..usize::BITS - bindings.receipts().len().leading_zeros() {
                                self.step(owner)?;
                            }
                            let binding = bindings
                                .receipt(selected.occurrence().item().node_id())
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            match binding.kind() {
                                PrecomposedVectorKind::InlineVector => {
                                    let local = selected.geometry().viewport();
                                    let viewport = Rect::new(
                                        plus(fragment.bounds().x(), local.x(), owner)?,
                                        plus(fragment.bounds().y(), local.y(), owner)?,
                                        local.width(),
                                        local.height(),
                                    );
                                    let inline = u32::try_from(inline)
                                        .map_err(|_| error(owner, E::RecordLimit))?;
                                    push(
                                        self,
                                        BookV2ImageSource::Vector(binding),
                                        Some(inline),
                                        viewport,
                                    )?;
                                }
                                PrecomposedVectorKind::MathVector => (),
                                _ => return Err(error(owner, E::ReceiptMismatch).into()),
                            }
                        }
                    }
                    ProductionBodyFragmentSource::NativeMathBlock { .. } => (),
                }
                index = index
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::RecordLimit))?;
            }
        }
        Ok(())
    }
    fn image_paint(
        &mut self,
        source: BookV2ImageSource<'p, 'a>,
        viewport: Rect,
    ) -> Result<(&'d AdmittedImage, BookV2ImagePaint), BookV2MathDisplayError> {
        let owner = source.owner();
        self.step(owner)?;
        let id = match source {
            BookV2ImageSource::Figure(f) => f.image_id(),
            BookV2ImageSource::Vector(v) => v.resource().image_id(),
        };
        let image = self
            .admitted
            .image(id)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let (content_key, scale_raw, color) = match source {
            BookV2ImageSource::Figure(f) => {
                if image.content_hash() != f.admitted_sha256()
                    || viewport.width() != f.width()
                    || viewport.height() != f.height()
                {
                    return Err(error(owner, E::ReceiptMismatch).into());
                }
                match f.media() {
                    ProductionFigureMedia::Raster {
                        pixel_width,
                        pixel_height,
                    } => {
                        if image.admitted_safe_vector().is_some()
                            || image.width().get() != pixel_width
                            || image.height().get() != pixel_height
                        {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                        return Ok((image, BookV2ImagePaint::Raster { viewport }));
                    }
                    ProductionFigureMedia::Svg {
                        content_key,
                        scale_raw,
                    } => {
                        if VectorContentKey::from_admitted(image).ok() != Some(content_key) {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                        (content_key, scale_raw, [0, 0, 0])
                    }
                }
            }
            BookV2ImageSource::Vector(v) => {
                let key = crate::precomposed_vector::content_key_for_resource(v.resource(), image)
                    .map_err(|_| error(owner, E::ReceiptMismatch))?;
                let (scale, paint) = match v.placement() {
                    PrecomposedVectorPlacementInput::Inline(p) => {
                        (p.scale().get().raw(), p.paint())
                    }
                    PrecomposedVectorPlacementInput::VectorFigure(p) => {
                        (p.scale().get().raw(), p.paint())
                    }
                    _ => return Err(error(owner, E::ReceiptMismatch).into()),
                };
                (key, scale, [paint.red(), paint.green(), paint.blue()])
            }
        };
        Ok((
            image,
            BookV2ImagePaint::Vector {
                content_key,
                viewport,
                scale_raw,
                matrix: placement_matrix(viewport, scale_raw),
                color,
            },
        ))
    }
    pub fn build_images(
        &mut self,
    ) -> Result<BookV2ImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let mut count = 0usize;
        self.visit_images(|_, _| {
            count = count
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?;
            Ok(())
        })?;
        take(
            &mut self.remaining,
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?,
            root,
        )?;
        let mut draws = Vec::new();
        draws
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        self.step(root)?;
        let mut fp = sha256(BOOK_V2_IMAGE_DISPLAY_ALGORITHM.as_bytes());
        fp = self.fold(fp, &self.source.fingerprint(), root)?;
        fp = self.fold(fp, &self.admitted.fingerprint(), root)?;
        self.visit_images(|this, draw| {
            let owner = draw.source.owner();
            let fragment = draw.fragment.fragment();
            let mut meta = [0; 49];
            meta[..4].copy_from_slice(&owner.get().to_be_bytes());
            meta[4..8].copy_from_slice(&fragment.page_index().to_be_bytes());
            meta[8..16].copy_from_slice(&(draw.index as u64).to_be_bytes());
            meta[16..24].copy_from_slice(&(draw.fragment.item_index() as u64).to_be_bytes());
            meta[24..32].copy_from_slice(
                &(draw.fragment.definition_index().unwrap_or(0) as u64).to_be_bytes(),
            );
            meta[32] = u8::from(draw.fragment.definition_index().is_some());
            meta[33..37].copy_from_slice(&draw.inline.unwrap_or(0).to_be_bytes());
            meta[37] = u8::from(draw.inline.is_some());
            meta[38..42].copy_from_slice(
                &draw
                    .cell
                    .map(|c| c.owner().get())
                    .unwrap_or(0)
                    .to_be_bytes(),
            );
            meta[42] = u8::from(draw.cell.is_some());
            meta[43] = u8::from(draw.repeated);
            meta[44..48].copy_from_slice(&draw.image.image_id().get().to_be_bytes());
            meta[48] = match draw.source {
                BookV2ImageSource::Figure(_) => 0,
                BookV2ImageSource::Vector(_) => 1,
            };
            fp = this.fold(fp, &meta, owner)?;
            fp = this.fold(fp, &draw.image.content_hash(), owner)?;
            if let BookV2ImageSource::Vector(v) = draw.source {
                fp = this.fold(fp, &v.fingerprint(), owner)?;
            }
            let alt = draw.source.alternative().as_bytes();
            fp = this.fold(fp, &(alt.len() as u64).to_be_bytes(), owner)?;
            for chunk in alt.chunks(104) {
                fp = this.fold(fp, chunk, owner)?;
            }
            let mut geometry = [0; 33];
            put_rect(&mut geometry[..32], draw.paint.viewport());
            geometry[32] = u8::from(matches!(draw.paint, BookV2ImagePaint::Vector { .. }));
            fp = this.fold(fp, &geometry, owner)?;
            if let BookV2ImagePaint::Vector {
                content_key,
                scale_raw,
                color,
                ..
            } = draw.paint
            {
                let mut vector = [0; 71];
                vector[..32].copy_from_slice(&content_key.source_sha256());
                vector[32..64].copy_from_slice(&content_key.ir_fingerprint());
                vector[64..68].copy_from_slice(&scale_raw.to_be_bytes());
                vector[68..].copy_from_slice(&color);
                fp = this.fold(fp, &vector, owner)?;
            }
            draws.push(draw);
            Ok(())
        })?;
        Ok(BookV2ImageDisplay {
            source: self.source,
            admitted: self.admitted,
            draws,
            fingerprint: fp,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }
}
