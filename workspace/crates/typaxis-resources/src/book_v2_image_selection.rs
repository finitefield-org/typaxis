//! Actual raster/vector selection, sharing payload identities but never uses.
use super::*;
use typaxis_display_list::book_v2::BookV2ImageUse;
use typaxis_resource_admission::{AdmittedImage, AdmittedImageMediaKind, VectorContentKey};

pub const BOOK_V2_IMAGE_SELECTION_ALGORITHM: &str = "typaxis.book-2-image-selection/1";
type Key = (u8, [u8; 32], &'static str, &'static str, [u8; 32]);
fn key(image: &AdmittedImage) -> Result<Key, E> {
    let kind = match image.media_kind() {
        AdmittedImageMediaKind::Png => 0,
        AdmittedImageMediaKind::JpegBaseline => 1,
        AdmittedImageMediaKind::SafeVector => 2,
        AdmittedImageMediaKind::SafeVector2 => 3,
    };
    let (parser, ir, fingerprint) = if kind >= 2 {
        let k = VectorContentKey::from_admitted(image).map_err(|_| E::Identity)?;
        (k.parser_id(), k.ir_id(), k.ir_fingerprint())
    } else {
        ("", "", [0; 32])
    };
    Ok((kind, image.content_hash(), parser, ir, fingerprint))
}
pub struct BookV2SelectedImage<'v> {
    image: &'v AdmittedImage,
    key: Key,
}
impl<'v> BookV2SelectedImage<'v> {
    pub fn image(&self) -> &'v AdmittedImage {
        self.image
    }
}
pub struct BookV2SelectedImageUse<'v> {
    paint: usize,
    usage: BookV2ImageUse<'v>,
    resource: usize,
}
impl<'v> BookV2SelectedImageUse<'v> {
    pub fn paint_index(&self) -> usize {
        self.paint
    }
    pub fn usage(&self) -> &BookV2ImageUse<'v> {
        &self.usage
    }
    pub fn resource_index(&self) -> usize {
        self.resource
    }
}
pub struct BookV2ImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    images: Vec<BookV2SelectedImage<'v>>,
    uses: Vec<BookV2SelectedImageUse<'v>>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
    prior: [u64; 3],
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn display(&self) -> &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.display
    }
    pub fn images(&self) -> &[BookV2SelectedImage<'v>] {
        &self.images
    }
    pub fn uses(&self) -> &[BookV2SelectedImageUse<'v>] {
        &self.uses
    }
    pub fn for_paint(&self, paint: usize) -> Option<&BookV2SelectedImageUse<'v>> {
        self.uses
            .binary_search_by_key(&paint, |u| u.paint)
            .ok()
            .and_then(|i| self.uses.get(i))
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    /// Prefix counters before this selection, excluding its builder record.
    /// A later merge can require the text branch to precede image allocation.
    pub fn prior_charges(&self) -> [u64; 3] {
        self.prior
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn select_images(
        &mut self,
    ) -> Result<BookV2ImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
        let prior = [
            self.records.checked_sub(1).ok_or(E::Identity)?,
            self.spool,
            self.work,
        ];
        let display = self.display;
        let mut count = 0usize;
        for paint in 0..display.paints().len() {
            self.step()?;
            if display.image_use(paint).map_err(|_| E::Identity)?.is_some() {
                count = count.checked_add(1).ok_or(E::Records)?;
            }
        }
        let records = count
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let bytes = count
            .checked_mul(
                std::mem::size_of::<BookV2SelectedImage<'v>>()
                    + std::mem::size_of::<BookV2SelectedImageUse<'v>>(),
            )
            .ok_or(E::Spool)?;
        self.closure_charge(records, bytes, 1)?;
        let mut images = Vec::new();
        let mut uses = Vec::new();
        images.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        uses.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        for paint in 0..display.paints().len() {
            self.step()?;
            if let Some(usage) = display.image_use(paint).map_err(|_| E::Identity)? {
                images.push(BookV2SelectedImage {
                    image: usage.image(),
                    key: key(usage.image())?,
                });
                uses.push(BookV2SelectedImageUse {
                    paint,
                    usage,
                    resource: 0,
                });
            }
        }
        self.sort_by_key(&mut images, 4, |i| (i.key, i.image.image_id().get()))?;
        let mut retained = 0;
        for index in 0..images.len() {
            self.step()?;
            if retained > 0 && images[retained - 1].key == images[index].key {
                let prior = images[retained - 1].image;
                let next = images[index].image;
                self.closure_charge(0, 0, next.bytes().len().div_ceil(64) + 1)?;
                if prior.bytes() != next.bytes()
                    || prior.width() != next.width()
                    || prior.height() != next.height()
                    || prior.decoded_bytes() != next.decoded_bytes()
                {
                    return Err(E::Identity);
                }
                continue;
            }
            images.swap(retained, index);
            retained += 1;
        }
        images.truncate(retained);
        let mut fingerprint = sha256(BOOK_V2_IMAGE_SELECTION_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &display.fingerprint())?;
        fingerprint = self.fold(fingerprint, &display.admitted().fingerprint())?;
        for image in &images {
            self.step()?;
            fingerprint = self.fold(fingerprint, &[image.key.0])?;
            fingerprint = self.fold(fingerprint, &image.image.image_id().get().to_be_bytes())?;
            fingerprint = self.fold(fingerprint, &image.key.1)?;
            fingerprint = self.fold(fingerprint, image.key.2.as_bytes())?;
            fingerprint = self.fold(fingerprint, image.key.3.as_bytes())?;
            fingerprint = self.fold(fingerprint, &image.key.4)?;
        }
        for usage in &mut uses {
            self.closure_charge(
                0,
                0,
                (usize::BITS - images.len().leading_zeros()) as usize + 1,
            )?;
            usage.resource = images
                .binary_search_by_key(&key(usage.usage.image())?, |i| i.key)
                .map_err(|_| E::Identity)?;
            let mut b = [0; 20];
            b[..8].copy_from_slice(&(usage.paint as u64).to_be_bytes());
            b[8..16].copy_from_slice(&(usage.resource as u64).to_be_bytes());
            b[16..].copy_from_slice(&usage.usage.image().image_id().get().to_be_bytes());
            fingerprint = self.fold(fingerprint, &b)?;
        }
        Ok(BookV2ImageSelection {
            prior,
            display,
            images,
            uses,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
}

#[path = "book_v2_raster_programs.rs"]
mod rasters;
pub use rasters::*;
