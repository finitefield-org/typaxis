//! Font-only object fragments. The later complete PDF owner must allocate a
//! disjoint range, bind page resources and validate the final object graph.
use super::*;
use crate::font_dictionary::{FontDictionaryFields, Role, Widths};
use crate::{FontObjectIds, ObjectId, PdfFontProgramKind};

pub const BOOK_V2_FONT_OBJECTS_ALGORITHM: &str = "typaxis.book-2-font-objects/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontObjectRole {
    Type0,
    CidFont,
    Descriptor,
    Program,
    ToUnicode,
    Auxiliary,
}
#[derive(Clone, Debug)]
pub struct BookV2FontObject {
    id: ObjectId,
    font: usize,
    role: BookV2FontObjectRole,
    range: Range<usize>,
}
impl BookV2FontObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn font_index(&self) -> usize {
        self.font
    }
    pub fn role(&self) -> BookV2FontObjectRole {
        self.role
    }
    pub fn byte_range(&self) -> Range<usize> {
        self.range.clone()
    }
}
pub struct BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'o BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    objects: Vec<BookV2FontObject>,
    bytes: Vec<u8>,
    first: u32,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(
        &self,
    ) -> &'o BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn objects(&self) -> &[BookV2FontObject] {
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
    pub fn font_object(&self, font: usize) -> Option<ObjectId> {
        self.objects.get(font.checked_mul(6)?).map(|o| o.id)
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
fn ids(first: u32) -> Result<FontObjectIds, E> {
    let id = |offset| {
        first
            .checked_add(offset)
            .and_then(ObjectId::new)
            .ok_or(E::Objects)
    };
    Ok(FontObjectIds {
        type0: id(0)?,
        cid_font: id(1)?,
        descriptor: id(2)?,
        font_program: id(3)?,
        to_unicode: id(4)?,
        auxiliary: id(5)?,
    })
}
fn encode_objects(
    source: &BookV2FontStreams<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    first: u32,
    out: &mut Encoder<'_>,
    mut objects: Option<&mut Vec<BookV2FontObject>>,
) -> Result<(), E> {
    use BookV2FontObjectRole as R;
    for (index, font) in source.source().fonts().iter().enumerate() {
        out.budget.step(256)?;
        let program = font.source();
        let cff = program.kind() == BookV2FontClosureKind::Cff1V2;
        let ids = ids(first
            .checked_add(
                u32::try_from(index.checked_mul(6).ok_or(E::Objects)?).map_err(|_| E::Objects)?,
            )
            .ok_or(E::Objects)?)?;
        let notdef = if cff {
            Some(u32::from(
                program
                    .advance(typaxis_font::OriginalGlyphId::new(0))
                    .ok_or(E::Identity)?,
            ))
        } else {
            None
        };
        let fields = FontDictionaryFields {
            kind: if cff {
                PdfFontProgramKind::OpenTypeCff1
            } else {
                PdfFontProgramKind::TrueTypeGlyf
            },
            name: program.postscript_name(),
            metrics: program.metrics(),
            ids,
            widths: Widths::Book {
                bindings: source.source().bindings(index).ok_or(E::Identity)?,
                notdef,
            },
        };
        for (id, role) in [
            (ids.type0, R::Type0),
            (ids.cid_font, R::CidFont),
            (ids.descriptor, R::Descriptor),
            (ids.font_program, R::Program),
            (ids.to_unicode, R::ToUnicode),
            (ids.auxiliary, R::Auxiliary),
        ] {
            let start = out.length;
            out.unsigned(u64::from(id.get()))?;
            out.extend(b" 0 obj\n")?;
            match role {
                R::Type0 => fields.write(Role::Type0, out)?,
                R::CidFont => fields.write(Role::CidFont, out)?,
                R::Descriptor => fields.write(Role::Descriptor, out)?,
                R::Program | R::ToUnicode | R::Auxiliary => {
                    let bytes = match role {
                        R::Program => program.bytes(),
                        R::ToUnicode => source.to_unicode(index).ok_or(E::Identity)?,
                        _ => source.auxiliary(index).ok_or(E::Identity)?,
                    };
                    out.extend(b"<< /Length ")?;
                    out.unsigned(bytes.len() as u64)?;
                    if role == R::Program {
                        if cff {
                            out.extend(b" /Subtype /OpenType")?;
                        } else {
                            out.extend(b" /Length1 ")?;
                            out.unsigned(bytes.len() as u64)?;
                        }
                    }
                    out.extend(b" >>\nstream\n")?;
                    out.extend(bytes)?;
                    out.extend(b"\nendstream")?;
                }
            }
            out.extend(b"\nendobj\n")?;
            if let Some(objects) = &mut objects {
                objects.push(BookV2FontObject {
                    id,
                    font: index,
                    role,
                    range: start..out.length,
                });
            }
        }
    }
    Ok(())
}
pub struct BookV2FontObjectBuilder<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'o BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    first: u32,
    credit_available: bool,
    budget: Budget,
}
impl<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontObjectBuilder<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        source: &'o BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        first_object: u32,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let inherited = BookV2FontStreamBuilder::new(
            source.source(),
            limits,
            max_work,
            prior_records.max(source.record_charge()),
            prior_spool.max(source.spool_charge()),
            prior_output.max(source.output_charge()),
            prior_work.max(source.work_steps()),
        )?;
        let count = source.fonts().len().checked_mul(6).ok_or(E::Objects)?;
        // Also keep the exclusive next-object number representable.
        let next = u64::from(first_object)
            .checked_add(count as u64)
            .ok_or(E::Objects)?;
        if first_object == 0
            || next > u64::from(u32::MAX)
            || next - 1 > u64::from(limits.base().get().max_pdf_objects)
        {
            return Err(E::Objects);
        }
        Ok(Self {
            source,
            first: first_object,
            credit_available: true,
            budget: inherited.budget,
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
    ) -> Result<BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
        let count = self.source.fonts().len().checked_mul(6).ok_or(E::Objects)?;
        let metadata = count
            .checked_mul(std::mem::size_of::<BookV2FontObject>())
            .ok_or(E::Spool)?;
        // ToUnicode and auxiliary bytes were already charged as final output
        // by the source owner. Copying them consumes spool again, not another
        // final-output charge. ActualText remains reserved for body content.
        let mut credit = 0usize;
        for index in 0..self.source.fonts().len() {
            self.budget.step(1)?;
            credit = credit
                .checked_add(self.source.to_unicode(index).ok_or(E::Identity)?.len())
                .and_then(|n| n.checked_add(self.source.auxiliary(index)?.len()))
                .ok_or(E::Output)?;
        }
        if !self.credit_available {
            credit = 0;
        }
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
        encode_objects(self.source, self.first, &mut counter, None)?;
        let length = counter.length;
        let extra = length.checked_sub(credit).ok_or(E::Identity)?;
        self.budget.reserve(
            count.checked_add(1).ok_or(E::Records)?,
            metadata.checked_add(length).ok_or(E::Spool)?,
            extra,
        )?;
        self.credit_available = false;
        self.budget.step(1)?;
        let mut bytes = Vec::new();
        let mut objects = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        objects
            .try_reserve_exact(count)
            .map_err(|_| E::Allocation)?;
        let mut encoder = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        encode_objects(self.source, self.first, &mut encoder, Some(&mut objects))?;
        if encoder.length != length || objects.len() != count {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fingerprint = sha256(BOOK_V2_FONT_OBJECTS_ALGORITHM.as_bytes());
        fingerprint = self.budget.fold(fingerprint, &self.source.fingerprint())?;
        fingerprint = self.budget.fold(fingerprint, &self.first.to_be_bytes())?;
        fingerprint = self.budget.fold(fingerprint, &sha256(&bytes))?;
        for object in &objects {
            let mut data = [0; 29];
            data[..4].copy_from_slice(&object.id.get().to_be_bytes());
            data[4..12].copy_from_slice(&(object.font as u64).to_be_bytes());
            data[12..20].copy_from_slice(&(object.range.start as u64).to_be_bytes());
            data[20..28].copy_from_slice(&(object.range.end as u64).to_be_bytes());
            data[28] = object.role as u8;
            fingerprint = self.budget.fold(fingerprint, &data)?;
        }
        Ok(BookV2FontObjects {
            source: self.source,
            objects,
            bytes,
            first: self.first,
            fingerprint,
            budget: self.budget,
        })
    }
}
