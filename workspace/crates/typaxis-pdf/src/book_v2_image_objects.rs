//! Actual image/Form object fragments; global numbering and page/semantic
//! closure remain the responsibility of the complete document owner.
use super::*;
use crate::{image_encoding, ObjectId};
use typaxis_resources::{ImageColorSpace, ImageEncoding};
pub const BOOK_V2_IMAGE_OBJECTS_ALGORITHM: &str = "typaxis.book-2-image-objects/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ImageObjectRole {
    Image,
    SoftMask,
    Form,
    ExtGState,
}
#[derive(Clone, Debug)]
pub struct BookV2ImageObject {
    id: ObjectId,
    image: usize,
    role: BookV2ImageObjectRole,
    state: Option<usize>,
    range: Range<usize>,
}
impl BookV2ImageObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn image_index(&self) -> usize {
        self.image
    }
    pub fn role(&self) -> BookV2ImageObjectRole {
        self.role
    }
    pub fn state_index(&self) -> Option<usize> {
        self.state
    }
    pub fn byte_range(&self) -> Range<usize> {
        self.range.clone()
    }
}
pub struct BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'j BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    objects: Vec<BookV2ImageObject>,
    images: Vec<Range<usize>>,
    bytes: Vec<u8>,
    first: u32,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'j BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn objects(&self) -> &[BookV2ImageObject] {
        &self.objects
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn first_object(&self) -> u32 {
        self.first
    }
    pub fn next_object(&self) -> u32 {
        self.first + self.objects.len() as u32
    }
    pub fn for_image(&self, index: usize) -> Option<&[BookV2ImageObject]> {
        self.objects.get(self.images.get(index)?.clone())
    }
    pub fn resource_object(&self, index: usize) -> Option<ObjectId> {
        self.for_image(index)?.first().map(|o| o.id)
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
}
pub struct BookV2ImageObjectBuilder<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'j BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    first: u32,
    count: usize,
    credit: usize,
    credit_available: bool,
    budget: Budget,
}
impl<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ImageObjectBuilder<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        source: &'j BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        first: u32,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.source().source().display();
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let base = limits.base().get();
        let records = prior_records
            .max(source.record_charge())
            .checked_add(1)
            .ok_or(E::Records)?;
        let spool = prior_spool.max(source.spool_charge());
        let output = prior_output.max(source.output_charge());
        let work = prior_work.max(source.work_steps());
        if records > base.max_fragments {
            return Err(E::Records);
        }
        if spool > base.max_spool_bytes {
            return Err(E::Spool);
        }
        if output > base.max_output_bytes {
            return Err(E::Output);
        }
        if work > max_work {
            return Err(E::Work);
        }
        let mut budget = Budget {
            max_records: base.max_fragments,
            max_spool: base.max_spool_bytes,
            max_output: base.max_output_bytes,
            max_work,
            records,
            spool,
            output,
            work,
        };
        let mut count = 0usize;
        let mut credit = 0usize;
        for index in 0..source.source().source().images().len() {
            budget.step(1)?;
            let n = match (source.source().program(index), source.program(index)) {
                (Some(r), None) => 1 + usize::from(r.alpha_mask().is_some()),
                (None, Some(v)) => {
                    credit = credit.checked_add(v.content().len()).ok_or(E::Output)?;
                    for state in 0..v.states().len() {
                        budget.step(1)?;
                        credit = credit
                            .checked_add(v.state_dictionary(state).ok_or(E::Identity)?.len())
                            .ok_or(E::Output)?;
                    }
                    v.states().len().checked_add(1).ok_or(E::Objects)?
                }
                _ => return Err(E::Identity),
            };
            count = count.checked_add(n).ok_or(E::Objects)?;
        }
        let next = u64::from(first)
            .checked_add(count as u64)
            .ok_or(E::Objects)?;
        if first == 0 || next > u64::from(u32::MAX) || next - 1 > u64::from(base.max_pdf_objects) {
            return Err(E::Objects);
        }
        if credit as u64 > source.output_charge() {
            return Err(E::Identity);
        }
        Ok(Self {
            source,
            first,
            count,
            credit,
            credit_available: true,
            budget,
        })
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
    pub fn build(
        &mut self,
    ) -> Result<BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
        let n = self.source.source().source().images().len();
        let metadata = self
            .count
            .checked_mul(std::mem::size_of::<BookV2ImageObject>())
            .and_then(|b| b.checked_add(n.checked_mul(std::mem::size_of::<Range<usize>>())?))
            .ok_or(E::Spool)?;
        let credit = if self.credit_available {
            self.credit
        } else {
            0
        };
        let output_limit = (self.budget.max_output - self.budget.output)
            .checked_add(credit as u64)
            .ok_or(E::Output)?;
        let spool_limit = (self.budget.max_spool - self.budget.spool)
            .checked_sub(metadata as u64)
            .ok_or(E::Spool)?;
        let mut counter = Encoder {
            budget: &mut self.budget,
            bytes: None,
            length: 0,
            output_limit,
            spool_limit,
        };
        encode(self.source, self.first, &mut counter, None, None)?;
        let length = counter.length;
        self.budget.reserve(
            self.count
                .checked_add(n)
                .and_then(|v| v.checked_add(1))
                .ok_or(E::Records)?,
            metadata.checked_add(length).ok_or(E::Spool)?,
            length.checked_sub(credit).ok_or(E::Identity)?,
        )?;
        self.credit_available = false;
        self.budget.step(1)?;
        let mut bytes = Vec::new();
        let mut objects = Vec::new();
        let mut images = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        objects
            .try_reserve_exact(self.count)
            .map_err(|_| E::Allocation)?;
        images.try_reserve_exact(n).map_err(|_| E::Allocation)?;
        let mut out = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        encode(
            self.source,
            self.first,
            &mut out,
            Some(&mut objects),
            Some(&mut images),
        )?;
        if out.length != length || objects.len() != self.count || images.len() != n {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fp = self.budget.fold(
            sha256(BOOK_V2_IMAGE_OBJECTS_ALGORITHM.as_bytes()),
            &self.source.fingerprint(),
        )?;
        fp = self.budget.fold(fp, &self.first.to_be_bytes())?;
        fp = self.budget.fold(fp, &sha256(&bytes))?;
        for o in &objects {
            let mut b = [0; 38];
            b[..4].copy_from_slice(&o.id.get().to_be_bytes());
            b[4..12].copy_from_slice(&(o.image as u64).to_be_bytes());
            b[12..20].copy_from_slice(&(o.range.start as u64).to_be_bytes());
            b[20..28].copy_from_slice(&(o.range.end as u64).to_be_bytes());
            b[28] = o.role as u8;
            b[29] = u8::from(o.state.is_some());
            b[30..].copy_from_slice(&(o.state.unwrap_or(0) as u64).to_be_bytes());
            fp = self.budget.fold(fp, &b)?;
        }
        for r in &images {
            let mut b = [0; 16];
            b[..8].copy_from_slice(&(r.start as u64).to_be_bytes());
            b[8..].copy_from_slice(&(r.end as u64).to_be_bytes());
            fp = self.budget.fold(fp, &b)?;
        }
        Ok(BookV2ImageObjects {
            source: self.source,
            objects,
            images,
            bytes,
            first: self.first,
            fingerprint: fp,
            budget: self.budget,
        })
    }
}
fn reference(out: &mut Encoder<'_>, id: u32) -> Result<(), E> {
    out.unsigned(id.into())?;
    out.extend(b" 0 R")
}
fn stream(out: &mut Encoder<'_>, bytes: &[u8]) -> Result<(), E> {
    out.extend(b" /Length ")?;
    out.unsigned(bytes.len() as u64)?;
    out.extend(b" >>\nstream\n")?;
    out.extend(bytes)?;
    out.extend(b"\nendstream")
}
fn encode(
    source: &BookV2VectorPrograms<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    first: u32,
    out: &mut Encoder<'_>,
    mut objects: Option<&mut Vec<BookV2ImageObject>>,
    mut images: Option<&mut Vec<Range<usize>>>,
) -> Result<(), E> {
    use BookV2ImageObjectRole as R;
    let mut next = first;
    for index in 0..source.source().source().images().len() {
        out.budget.step(1)?;
        let before = (next - first) as usize;
        let raster = source.source().program(index);
        let vector = source.program(index);
        let count = match (raster, vector) {
            (Some(r), None) => 1 + usize::from(r.alpha_mask().is_some()),
            (None, Some(v)) => 1 + v.states().len(),
            _ => return Err(E::Identity),
        };
        for part in 0..count {
            out.budget.step(1)?;
            let id = ObjectId::new(next).ok_or(E::Objects)?;
            let start = out.length;
            out.unsigned(next.into())?;
            out.extend(b" 0 obj\n")?;
            let (role, state) = if let Some(r) = raster {
                let mask = if part == 0 {
                    None
                } else {
                    Some(r.alpha_mask().ok_or(E::Identity)?)
                };
                let bytes = mask.map_or_else(|| r.bytes(), |m| m.encoded_bytes.as_slice());
                let color = if mask.is_some() {
                    image_encoding::Color::Gray
                } else {
                    match r.color_space() {
                        ImageColorSpace::Gray => image_encoding::Color::Gray,
                        ImageColorSpace::Rgb => image_encoding::Color::Rgb,
                        _ => return Err(E::Identity),
                    }
                };
                if let Some(m) = mask {
                    if m.width.get() != r.width()
                        || m.height.get() != r.height()
                        || m.bits_per_component != 8
                        || m.encoding != ImageEncoding::Flate
                    {
                        return Err(E::Identity);
                    }
                }
                out.extend(b"<<")?;
                image_encoding::raster_fields(out, r.width(), r.height(), color, 8)?;
                let transform = if mask.is_none() && r.encoding() == ImageEncoding::Jpeg {
                    if r.jpeg().is_none() || r.alpha_mask().is_some() {
                        return Err(E::Identity);
                    }
                    Some(u8::from(r.color_space() == ImageColorSpace::Rgb))
                } else {
                    if mask.is_none() && r.encoding() != ImageEncoding::Flate {
                        return Err(E::Identity);
                    }
                    None
                };
                image_encoding::filter(out, transform)?;
                if part == 0 && r.alpha_mask().is_some() {
                    out.extend(b" /SMask ")?;
                    reference(out, next.checked_add(1).ok_or(E::Objects)?)?;
                }
                stream(out, bytes)?;
                (
                    if mask.is_some() {
                        R::SoftMask
                    } else {
                        R::Image
                    },
                    None,
                )
            } else {
                let v = vector.ok_or(E::Identity)?;
                if part == 0 {
                    image_encoding::form_header(out, v.bbox())?;
                    for state in 0..v.states().len() {
                        out.budget.step(1)?;
                        out.extend(b" /GS")?;
                        out.unsigned(state as u64)?;
                        out.push(b' ')?;
                        reference(
                            out,
                            next.checked_add(u32::try_from(state + 1).map_err(|_| E::Objects)?)
                                .ok_or(E::Objects)?,
                        )?;
                    }
                    out.extend(b" >> >>")?;
                    stream(out, v.content())?;
                    (R::Form, None)
                } else {
                    out.extend(v.state_dictionary(part - 1).ok_or(E::Identity)?)?;
                    (R::ExtGState, Some(part - 1))
                }
            };
            out.extend(b"\nendobj\n")?;
            if let Some(objects) = &mut objects {
                objects.push(BookV2ImageObject {
                    id,
                    image: index,
                    role,
                    state,
                    range: start..out.length,
                });
            }
            next = next.checked_add(1).ok_or(E::Objects)?;
        }
        if let Some(images) = &mut images {
            images.push(before..(next - first) as usize);
        }
    }
    Ok(())
}
