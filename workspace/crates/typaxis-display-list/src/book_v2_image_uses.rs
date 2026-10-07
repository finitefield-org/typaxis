//! Image use can only be issued from an actual ordered body paint.
use super::*;
use typaxis_resource_admission::{AdmittedImage, AdmittedImageMediaKind};

pub struct BookV2ImageUse<'v> {
    paint: BookV2BodyPaintIndex,
    image: &'v AdmittedImage,
    owner: NodeId,
    page: u32,
    geometry: BookV2ImagePaint,
}
impl<'v> BookV2ImageUse<'v> {
    pub fn paint(&self) -> BookV2BodyPaintIndex {
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
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn image_use(
        &self,
        paint_index: usize,
    ) -> Result<Option<BookV2ImageUse<'_>>, BookV2MathDisplayError> {
        let Some(paint) = self.paints().get(paint_index).copied() else {
            return Ok(None);
        };
        let (image, owner, page, geometry) = match paint {
            BookV2BodyPaintIndex::Image(i) => {
                let d = &self.images().draws()[i];
                (
                    d.image(),
                    d.source().owner(),
                    d.fragment().fragment().page_index(),
                    d.paint(),
                )
            }
            BookV2BodyPaintIndex::Math(i) => {
                let d = &self.math().draws()[i];
                let BookV2MathPaint::Vector(v) = d.paint() else {
                    return Ok(None);
                };
                let BookV2BodyMathSource::Vector(binding) = d.terminal().source() else {
                    return Err(error(d.terminal().source().owner(), E::ReceiptMismatch).into());
                };
                let image = self
                    .admitted()
                    .image(binding.resource().image_id())
                    .ok_or_else(|| error(binding.node_id(), E::ReceiptMismatch))?;
                (
                    image,
                    binding.node_id(),
                    d.terminal().page_index(),
                    BookV2ImagePaint::Vector {
                        content_key: v.content_key(),
                        viewport: v.viewport(),
                        scale_raw: v.scale_raw(),
                        matrix: v.matrix(),
                        color: v.color(),
                    },
                )
            }
            _ => return Ok(None),
        };
        if !self
            .admitted()
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
        Ok(Some(BookV2ImageUse {
            paint,
            image,
            owner,
            page,
            geometry,
        }))
    }
}
