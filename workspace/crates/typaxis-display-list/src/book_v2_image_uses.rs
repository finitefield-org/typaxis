//! Image use can only be issued from an actual ordered body or column paint.
use super::*;
use typaxis_resource_admission::{AdmittedImage, AdmittedImageMediaKind};

pub struct BookV2ImageUseData<'v, P: Copy> {
    paint: P,
    image: &'v AdmittedImage,
    owner: NodeId,
    page: u32,
    geometry: BookV2ImagePaint,
}
pub type BookV2ImageUse<'v> = BookV2ImageUseData<'v, BookV2BodyPaintIndex>;
/// An original column paint cannot authorize a body image consumer.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2ColumnImageUse, BookV2ImageUse};
/// fn body<'v>(v: BookV2ColumnImageUse<'v>) -> BookV2ImageUse<'v> { v }
/// ```
pub type BookV2ColumnImageUse<'v> = BookV2ImageUseData<'v, BookV2ColumnPaintIndex>;
impl<'v, P: Copy> BookV2ImageUseData<'v, P> {
    pub fn paint(&self) -> P {
        self.paint
    }
    pub fn image(&self) -> &'v AdmittedImage {
        self.image
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn geometry(&self) -> BookV2ImagePaint {
        self.geometry
    }
    pub fn vector_content_key(&self) -> Option<VectorContentKey> {
        match self.geometry {
            BookV2ImagePaint::Vector { content_key, .. } => Some(content_key),
            _ => None,
        }
    }
}

struct ImageUseSource<'v> {
    image: &'v AdmittedImage,
    owner: NodeId,
    page: u32,
    geometry: BookV2ImagePaint,
}
fn original_image_source<'v>(draw: &'v BookV2ImageDraw<'_, '_, '_, '_>) -> ImageUseSource<'v> {
    ImageUseSource {
        image: draw.image(),
        owner: draw.source().owner(),
        page: draw.fragment().fragment().page_index(),
        geometry: draw.paint(),
    }
}
fn vector_math_source<'v>(
    paint: &BookV2MathPaint,
    source: BookV2BodyMathSource<'_, '_>,
    page: u32,
    admitted: &'v AdmittedProductionResourceLedgerV3,
) -> Result<Option<ImageUseSource<'v>>, BookV2MathDisplayError> {
    let BookV2MathPaint::Vector(vector) = paint else {
        return Ok(None);
    };
    let BookV2BodyMathSource::Vector(binding) = source else {
        return Err(error(source.owner(), E::ReceiptMismatch).into());
    };
    let image = admitted
        .image(binding.resource().image_id())
        .ok_or_else(|| error(binding.node_id(), E::ReceiptMismatch))?;
    Ok(Some(ImageUseSource {
        image,
        owner: binding.node_id(),
        page,
        geometry: BookV2ImagePaint::Vector {
            content_key: vector.content_key(),
            viewport: vector.viewport(),
            scale_raw: vector.scale_raw(),
            matrix: vector.matrix(),
            color: vector.color(),
        },
    }))
}
// The issuer is private. Only a concrete authentic display's typed paint match
// supplies a source; original admitted pointer, media and vector key checks are
// identical for body and column uses.
fn issue_image_use<'v, P: Copy>(
    paint: P,
    source: ImageUseSource<'v>,
    admitted: &AdmittedProductionResourceLedgerV3,
) -> Result<BookV2ImageUseData<'v, P>, BookV2MathDisplayError> {
    let ImageUseSource {
        image,
        owner,
        page,
        geometry,
    } = source;
    if !admitted
        .image(image.image_id())
        .is_some_and(|original| std::ptr::eq(original, image))
    {
        return Err(error(owner, E::ReceiptMismatch).into());
    }
    match geometry {
        BookV2ImagePaint::Raster { .. } => {
            if !matches!(
                image.media_kind(),
                AdmittedImageMediaKind::Png | AdmittedImageMediaKind::JpegBaseline
            ) {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
        }
        BookV2ImagePaint::Vector { content_key, .. } => {
            if VectorContentKey::from_admitted(image).ok() != Some(content_key) {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
        }
    }
    Ok(BookV2ImageUseData {
        paint,
        image,
        owner,
        page,
        geometry,
    })
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn image_use(
        &self,
        paint_index: usize,
    ) -> Result<Option<BookV2ImageUse<'_>>, BookV2MathDisplayError> {
        let Some(paint) = self.paints().get(paint_index).copied() else {
            return Ok(None);
        };
        let source = match paint {
            BookV2BodyPaintIndex::Image(i) => original_image_source(&self.images().draws()[i]),
            BookV2BodyPaintIndex::Math(i) => {
                let d = &self.math().draws()[i];
                let Some(source) = vector_math_source(
                    d.paint(),
                    d.terminal().source(),
                    d.terminal().page_index(),
                    self.admitted(),
                )?
                else {
                    return Ok(None);
                };
                source
            }
            _ => return Ok(None),
        };
        issue_image_use(paint, source, self.admitted()).map(Some)
    }
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn image_use(
        &self,
        paint_index: usize,
    ) -> Result<Option<BookV2ColumnImageUse<'_>>, BookV2MathDisplayError> {
        let Some(paint) = self.paints().get(paint_index).copied() else {
            return Ok(None);
        };
        let source = match paint {
            BookV2ColumnPaintIndex::Image(i) => original_image_source(&self.images().draws()[i]),
            BookV2ColumnPaintIndex::Math(i) => {
                let draw = &self.math().draws()[i];
                let Some(source) = vector_math_source(
                    draw.paint(),
                    draw.terminal().source(),
                    draw.terminal().page_index(),
                    self.admitted(),
                )?
                else {
                    return Ok(None);
                };
                source
            }
            _ => return Ok(None),
        };
        issue_image_use(paint, source, self.admitted()).map(Some)
    }
}
