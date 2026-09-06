//! Measured ordinary raster figures, bound to the admitted source and flow.
use super::*;
use typaxis_core::ImageResourceId;
use typaxis_resource_admission::AdmittedImageMediaKind;
use typaxis_style::MachineFigureWidth;
use typaxis_syntax::ProductionFigure;

pub struct ProductionPreparedRasterFigure<'a> {
    source: &'a ProductionFigure<'a>,
    source_index: u32,
    admitted_sha256: [u8; 32],
    pixel_width: u32,
    pixel_height: u32,
    width: PositiveLength,
    height: PositiveLength,
}
impl<'a> ProductionPreparedRasterFigure<'a> {
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
    pub const fn pixel_width(&self) -> u32 {
        self.pixel_width
    }
    pub const fn pixel_height(&self) -> u32 {
        self.pixel_height
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}
pub(super) fn prepare_figures<'a>(
    flow: &'a ProductionTextFlow<'a>,
    admitted: &AdmittedResourceLedger,
    charge: &mut u64,
    maximum: u64,
) -> Result<Vec<ProductionPreparedRasterFigure<'a>>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let mut output = Vec::new();
    for (index, source) in flow.figures().iter().enumerate() {
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
        if !matches!(
            image.media_kind(),
            AdmittedImageMediaKind::Png | AdmittedImageMediaKind::JpegBaseline
        ) {
            return Err(error(owner, E::UnsupportedFigureMedia));
        }
        let MachineFigureWidth::Length(width) = source.style().block_style().width() else {
            return Err(error(owner, E::MissingFigureWidth));
        };
        // Physical width is fixed point; preserve pixel aspect with one exact
        // ties-to-even conversion, never independent X/Y authoring scales.
        let height = Length::from_rational_pdf_points(
            i128::from(width.get().raw()) * i128::from(image.height().get()),
            i128::from(image.width().get()) * 65_536,
        )
        .ok()
        .and_then(PositiveLength::new)
        .ok_or_else(|| error(owner, E::InvalidFigureGeometry))?;
        output
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        output.push(ProductionPreparedRasterFigure {
            source,
            source_index: u32::try_from(index).map_err(|_| error(owner, E::UnitLimit))?,
            admitted_sha256: image.content_hash(),
            pixel_width: image.width().get(),
            pixel_height: image.height().get(),
            width,
            height,
        });
    }
    Ok(output)
}
