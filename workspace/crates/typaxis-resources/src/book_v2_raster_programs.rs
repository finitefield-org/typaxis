//! PNG pixels and JPEG normalized streams from selected book-2 images.
use super::*;
use crate::{ImageColorSpace, ImageEncoderOutput, ImageEncoding, ResourceError};
use typaxis_resource_admission::{JpegAdmissionAttestation, JpegColorKind};

pub const BOOK_V2_RASTER_PROGRAMS_ALGORITHM: &str = "typaxis.book-2-raster-programs/1";
#[derive(Debug)]
pub enum BookV2RasterError {
    Budget(E),
    Decode(ResourceError),
}
impl From<E> for BookV2RasterError {
    fn from(e: E) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2RasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 raster: {self:?}")
    }
}
impl std::error::Error for BookV2RasterError {}
enum Payload<'v> {
    Png(ImageEncoderOutput),
    Jpeg(&'v JpegAdmissionAttestation),
}
pub struct BookV2RasterProgram<'i, 'v> {
    source: &'i BookV2SelectedImage<'v>,
    payload: Payload<'v>,
    sha256: [u8; 32],
    fingerprint: [u8; 32],
}
impl<'i, 'v> BookV2RasterProgram<'i, 'v> {
    pub fn source(&self) -> &'i BookV2SelectedImage<'v> {
        self.source
    }
    pub fn bytes(&self) -> &[u8] {
        match &self.payload {
            Payload::Png(p) => &p.encoded_bytes,
            Payload::Jpeg(j) => j.normalized_bytes(),
        }
    }
    pub fn width(&self) -> u32 {
        self.source.image().width().get()
    }
    pub fn height(&self) -> u32 {
        self.source.image().height().get()
    }
    pub fn color_space(&self) -> ImageColorSpace {
        match &self.payload {
            Payload::Png(p) => p.color_space,
            Payload::Jpeg(j) => match j.color_kind() {
                JpegColorKind::Grayscale => ImageColorSpace::Gray,
                JpegColorKind::YCbCr => ImageColorSpace::Rgb,
            },
        }
    }
    pub fn encoding(&self) -> ImageEncoding {
        match self.payload {
            Payload::Png(_) => ImageEncoding::Flate,
            Payload::Jpeg(_) => ImageEncoding::Jpeg,
        }
    }
    pub fn alpha_mask(&self) -> Option<&crate::AlphaMaskEncoderOutput> {
        match &self.payload {
            Payload::Png(p) => p.alpha_mask.as_ref(),
            _ => None,
        }
    }
    pub fn jpeg(&self) -> Option<&'v JpegAdmissionAttestation> {
        match self.payload {
            Payload::Jpeg(j) => Some(j),
            _ => None,
        }
    }
    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
pub struct BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'i BookV2ImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    programs: Vec<Option<BookV2RasterProgram<'i, 'v>>>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'i BookV2ImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn program(&self, image: usize) -> Option<&BookV2RasterProgram<'i, 'v>> {
        self.programs.get(image)?.as_ref()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
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
    pub fn write_raster_programs<'i>(
        &mut self,
        source: &'i BookV2ImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2RasterError>
    {
        if !std::ptr::eq(self.display, source.display()) {
            return Err(E::Identity.into());
        }
        self.records = self.records.max(source.record_charge());
        self.spool = self.spool.max(source.spool_charge());
        self.work = self.work.max(source.work_steps());
        self.closure_charge(
            source.images().len().checked_add(1).ok_or(E::Records)?,
            source
                .images()
                .len()
                .checked_mul(std::mem::size_of::<Option<BookV2RasterProgram<'i, 'v>>>())
                .ok_or(E::Spool)?,
            1,
        )?;
        let mut programs = Vec::new();
        programs
            .try_reserve_exact(source.images().len())
            .map_err(|_| E::Allocation)?;
        let mut fingerprint = sha256(BOOK_V2_RASTER_PROGRAMS_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &source.fingerprint())?;
        for (index, selected) in source.images().iter().enumerate() {
            self.step()?;
            let image = selected.image();
            let payload = match image.media_kind() {
                AdmittedImageMediaKind::SafeVector | AdmittedImageMediaKind::SafeVector2 => {
                    programs.push(None);
                    continue;
                }
                AdmittedImageMediaKind::JpegBaseline => {
                    let j = image.jpeg_attestation().ok_or(E::Identity)?;
                    self.closure_charge(1, 0, j.normalized_bytes().len().div_ceil(64) + 1)?;
                    if j.image_id() != image.image_id()
                        || j.source_sha256() != image.content_hash()
                        || j.width() != image.width()
                        || j.height() != image.height()
                        || j.decoded_byte_length() != image.decoded_bytes()
                        || sha256(j.normalized_bytes()) != j.normalized_sha256()
                    {
                        return Err(E::Identity.into());
                    }
                    Payload::Jpeg(j)
                }
                AdmittedImageMediaKind::Png => {
                    let decoded = usize::try_from(image.decoded_bytes()).map_err(|_| E::Spool)?;
                    let decoder = decoded
                        .checked_add(image.bytes().len())
                        .and_then(|n| n.checked_add(65536))
                        .ok_or(E::Spool)?;
                    // Reserve bounded PNG decoder and color/alpha buffers
                    // before decoding. Each deflater is charged separately.
                    let workspace = decoded
                        .checked_mul(2)
                        .and_then(|n| n.checked_add(decoder))
                        .ok_or(E::Spool)?;
                    let work = decoded
                        .checked_add(image.bytes().len())
                        .and_then(|n| n.checked_add(65536))
                        .ok_or(E::Work)?;
                    self.closure_charge(8, workspace, work)?;
                    let mut p = crate::decode_png_bytes_for_pdf_with_workspace(
                        image.image_id(),
                        image.content_hash(),
                        image.bytes(),
                        image.width(),
                        image.height(),
                        image.decoded_bytes(),
                        decoder,
                    )
                    .map_err(BookV2RasterError::Decode)?;
                    p.encoded_bytes = self.compress_raster(&p.encoded_bytes)?;
                    p.encoding = ImageEncoding::Flate;
                    if let Some(alpha) = &mut p.alpha_mask {
                        alpha.encoded_bytes = self.compress_raster(&alpha.encoded_bytes)?;
                        alpha.encoding = ImageEncoding::Flate;
                    }
                    Payload::Png(p)
                }
            };
            let bytes = match &payload {
                Payload::Png(p) => p.encoded_bytes.as_slice(),
                Payload::Jpeg(j) => j.normalized_bytes(),
            };
            self.closure_charge(0, 0, bytes.len().div_ceil(64) + 1)?;
            let hash = sha256(bytes);
            let mut fp = sha256(BOOK_V2_RASTER_PROGRAMS_ALGORITHM.as_bytes());
            fp = self.fold(fp, &source.fingerprint())?;
            fp = self.fold(fp, &(index as u64).to_be_bytes())?;
            fp = self.fold(fp, &hash)?;
            if let Payload::Png(p) = &payload {
                if let Some(alpha) = &p.alpha_mask {
                    self.closure_charge(0, 0, alpha.encoded_bytes.len().div_ceil(64) + 1)?;
                    fp = self.fold(fp, &sha256(&alpha.encoded_bytes))?;
                }
            }
            fingerprint = self.fold(fingerprint, &fp)?;
            programs.push(Some(BookV2RasterProgram {
                source: selected,
                payload,
                sha256: hash,
                fingerprint: fp,
            }));
        }
        Ok(BookV2RasterPrograms {
            source,
            programs,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
    fn compress_raster(&mut self, bytes: &[u8]) -> Result<Vec<u8>, BookV2RasterError> {
        let work = bytes
            .len()
            .checked_mul(128)
            .and_then(|n| n.checked_add(65536))
            .ok_or(E::Work)?;
        // The pinned level-6 backend has at most 128 hash-chain probes;
        // every encoder instance gets its own fixed-workspace reservation.
        self.closure_charge(1, 1_048_576, work)?;
        let remaining = self.maximum_spool.checked_sub(self.spool).ok_or(E::Spool)?;
        let mut failure = None;
        let result = crate::production_rasters::compress_with_charge(bytes, remaining, &mut |n| {
            self.closure_charge(1, n, n.div_ceil(64) + 1).map_err(|e| {
                failure = Some(e);
                ResourceError::ResourceLimit
            })
        });
        if let Some(error) = failure {
            return Err(error.into());
        }
        result.map_err(BookV2RasterError::Decode)
    }
}
