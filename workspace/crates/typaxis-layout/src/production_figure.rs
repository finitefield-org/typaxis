//! Measured ordinary figures, bound to the admitted source and flow.
use super::*;
use typaxis_core::ImageResourceId;
use typaxis_resource_admission::AdmittedImageMediaKind;
use typaxis_style::MachineFigureWidth;
use typaxis_syntax::ProductionFigure;

/// Media-specific dimensions; SVG dimensions are physical fixed-point lengths,
/// never the admission ledger's raster placeholder dimensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFigureMedia {
    Raster {
        pixel_width: u32,
        pixel_height: u32,
    },
    Svg {
        content_key: typaxis_resource_admission::VectorContentKey,
        scale_raw: i32,
    },
}
pub struct ProductionPreparedFigure<'a> {
    source: &'a ProductionFigure<'a>,
    source_index: u32,
    admitted_sha256: [u8; 32],
    media: ProductionFigureMedia,
    width: PositiveLength,
    height: PositiveLength,
}
impl<'a> ProductionPreparedFigure<'a> {
    pub const fn source(&self) -> &'a ProductionFigure<'a> {
        self.source
    }
    pub const fn source_index(&self) -> u32 {
        self.source_index
    }
    pub const fn owner(&self) -> NodeId {
        self.source.owner()
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.source.image_id()
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn media(&self) -> ProductionFigureMedia {
        self.media
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}
pub(super) fn prepare_figures<'a>(
    flow: InlineFlow<'a>,
    admitted: InlineImages<'_>,
    charge: &mut u64,
    maximum: u64,
) -> Result<Vec<ProductionPreparedFigure<'a>>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let mut output = Vec::new();
    for (index, source) in flow_call!(flow, figures()).iter().enumerate() {
        let owner = source.owner();
        *charge = charge
            .checked_add(1)
            .filter(|n| *n <= maximum)
            .ok_or_else(|| error(owner, E::UnitLimit))?;
        if source.placement() != "block" {
            return Err(error(owner, E::PendingFigurePlacement));
        }
        let image = admitted
            .image(source.image_id())
            .ok_or_else(|| error(owner, E::MissingFigureImage))?;
        let (media, width, height) = if let Some(ir) = image.admitted_safe_vector() {
            let intrinsic = ir.intrinsic_width();
            let requested = match source.style().block_style().width() {
                MachineFigureWidth::Auto => intrinsic,
                MachineFigureWidth::Length(width) => width,
            };
            // One uniform 16.16 scale controls both PDF axes. Quantize once;
            // selected bounds use precisely the same ties-even multiplication.
            // Match the existing safe-vector fit rule: choose the largest
            // representable uniform scale that does not exceed the width.
            let scale_raw = i32::try_from(
                i128::from(requested.get().raw()) * 65_536 / i128::from(intrinsic.get().raw()),
            )
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(|| error(owner, E::InvalidFigureGeometry))?;
            let scaled = |value: PositiveLength| {
                Length::from_rational_pdf_points(
                    i128::from(value.get().raw()) * i128::from(scale_raw),
                    65_536i128 * 65_536,
                )
                .ok()
                .and_then(PositiveLength::new)
                .ok_or_else(|| error(owner, E::InvalidFigureGeometry))
            };
            let width = scaled(intrinsic)?;
            (
                ProductionFigureMedia::Svg {
                    content_key: typaxis_resource_admission::VectorContentKey::from_admitted(image)
                        .map_err(|_| error(owner, E::InvalidFigureGeometry))?,
                    scale_raw,
                },
                width,
                scaled(ir.intrinsic_height())?,
            )
        } else {
            if !matches!(
                image.media_kind(),
                AdmittedImageMediaKind::Png | AdmittedImageMediaKind::JpegBaseline
            ) {
                return Err(error(owner, E::UnsupportedFigureMedia));
            }
            let MachineFigureWidth::Length(width) = source.style().block_style().width() else {
                return Err(error(owner, E::MissingFigureWidth));
            };
            let height = Length::from_rational_pdf_points(
                i128::from(width.get().raw()) * i128::from(image.height().get()),
                i128::from(image.width().get()) * 65_536,
            )
            .ok()
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::InvalidFigureGeometry))?;
            (
                ProductionFigureMedia::Raster {
                    pixel_width: image.width().get(),
                    pixel_height: image.height().get(),
                },
                width,
                height,
            )
        };
        output
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        output.push(ProductionPreparedFigure {
            source,
            source_index: u32::try_from(index).map_err(|_| error(owner, E::UnitLimit))?,
            admitted_sha256: image.content_hash(),
            media,
            width,
            height,
        });
    }
    Ok(output)
}
