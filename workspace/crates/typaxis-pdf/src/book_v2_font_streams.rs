//! Real font/extraction payloads from sealed book-2 CID plans. Object numbers
//! and marked-content authority are assigned by subsequent PDF stages.
use crate::font_encoding::{self, Sink};
use std::ops::Range;
use typaxis_core::{sha256, M4EffectiveResourceLimits};
use typaxis_display_list::book_v2::BookV2FontUseText;
use typaxis_resources::book_v2::{BookV2CidPlans, BookV2FontClosureKind};

pub const BOOK_V2_FONT_STREAMS_ALGORITHM: &str = "typaxis.book-2-font-streams/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontStreamError {
    Identity,
    Records,
    Spool,
    Output,
    Work,
    Allocation,
    Objects,
    Vector,
    Depth,
}
use BookV2FontStreamError as E;
impl std::fmt::Display for E {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 font streams: {self:?}")
    }
}
impl std::error::Error for E {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontAuxiliaryKind {
    TrueTypeCidToGid,
    CffCidSet,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2FontStream {
    font: usize,
    to_unicode: Range<usize>,
    auxiliary: Range<usize>,
    kind: BookV2FontAuxiliaryKind,
}
impl BookV2FontStream {
    pub fn font_index(&self) -> usize {
        self.font
    }
    pub fn auxiliary_kind(&self) -> BookV2FontAuxiliaryKind {
        self.kind
    }
}
pub struct BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'z BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    fonts: Vec<BookV2FontStream>,
    actual_text: Vec<Option<Range<usize>>>,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'z BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn fonts(&self) -> &[BookV2FontStream] {
        &self.fonts
    }
    pub fn to_unicode(&self, font: usize) -> Option<&[u8]> {
        self.bytes.get(self.fonts.get(font)?.to_unicode.clone())
    }
    pub fn auxiliary(&self, font: usize) -> Option<&[u8]> {
        self.bytes.get(self.fonts.get(font)?.auxiliary.clone())
    }
    /// A complete BOM-prefixed UTF-16BE PDF hex-string token. The later
    /// marked-content owner decides the actual replacement scope.
    pub fn actual_text(&self, usage: usize) -> Option<&[u8]> {
        self.bytes.get(self.actual_text.get(usage)?.clone()?)
    }
    pub fn byte_length(&self) -> usize {
        self.bytes.len()
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
#[derive(Clone, Copy)]
struct Budget {
    max_records: u64,
    max_spool: u64,
    max_output: u64,
    max_work: u64,
    records: u64,
    spool: u64,
    output: u64,
    work: u64,
}
impl Budget {
    fn step(&mut self, n: usize) -> Result<(), E> {
        let n = u64::try_from(n).map_err(|_| E::Work)?;
        if n > self.max_work.saturating_sub(self.work) {
            self.work = self.max_work;
            return Err(E::Work);
        }
        self.work += n;
        Ok(())
    }
    fn reserve(&mut self, records: usize, spool: usize, output: usize) -> Result<(), E> {
        let records = self
            .records
            .checked_add(records as u64)
            .filter(|n| *n <= self.max_records)
            .ok_or(E::Records)?;
        let spool = self
            .spool
            .checked_add(spool as u64)
            .filter(|n| *n <= self.max_spool)
            .ok_or(E::Spool)?;
        let output = self
            .output
            .checked_add(output as u64)
            .filter(|n| *n <= self.max_output)
            .ok_or(E::Output)?;
        self.records = records;
        self.spool = spool;
        self.output = output;
        Ok(())
    }
    fn fold(&mut self, prior: [u8; 32], next: &[u8]) -> Result<[u8; 32], E> {
        self.step((32 + next.len()).div_ceil(64))?;
        let mut bytes = [0; 128];
        bytes[..32].copy_from_slice(&prior);
        bytes[32..32 + next.len()].copy_from_slice(next);
        Ok(sha256(&bytes[..32 + next.len()]))
    }
}
struct Encoder<'a> {
    budget: &'a mut Budget,
    bytes: Option<&'a mut Vec<u8>>,
    length: usize,
    output_limit: u64,
    spool_limit: u64,
}
impl Sink for Encoder<'_> {
    type Error = E;
    fn extend(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.budget.step(bytes.len().div_ceil(64) + 1)?;
        let next = self.length.checked_add(bytes.len()).ok_or(E::Output)?;
        if next as u64 > self.output_limit {
            return Err(E::Output);
        }
        if next as u64 > self.spool_limit {
            return Err(E::Spool);
        }
        if let Some(out) = &mut self.bytes {
            if next > out.capacity() {
                return Err(E::Identity);
            }
            out.extend_from_slice(bytes);
        }
        self.length = next;
        Ok(())
    }
}
fn encode(
    source: &BookV2CidPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    output: &mut Encoder<'_>,
    mut fonts: Option<&mut Vec<BookV2FontStream>>,
    mut actual: Option<&mut Vec<Option<Range<usize>>>>,
) -> Result<(), E> {
    for (index, font) in source.fonts().iter().enumerate() {
        let bindings = source.bindings(index).ok_or(E::Identity)?;
        // Cloneable filtering visits at most twice for 100-entry chunks.
        output.budget.step(
            bindings
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or(E::Work)?,
        )?;
        let start = output.length;
        font_encoding::to_unicode(
            bindings
                .iter()
                .filter_map(|b| b.unicode().map(|c| (b.cid().get(), std::iter::once(c)))),
            output,
        )?;
        let split = output.length;
        let kind = match font.source().kind() {
            BookV2FontClosureKind::TrueType => {
                font_encoding::gid_map(output, bindings.iter().map(|b| b.subset_gid().get()))?;
                BookV2FontAuxiliaryKind::TrueTypeCidToGid
            }
            BookV2FontClosureKind::Cff1V2 => {
                let count = bindings.len().checked_add(1).ok_or(E::Identity)?;
                if font.source().source().glyphs().len() != count {
                    return Err(E::Identity);
                }
                font_encoding::cid_set(output, count)?;
                BookV2FontAuxiliaryKind::CffCidSet
            }
        };
        if let Some(fonts) = &mut fonts {
            fonts.push(BookV2FontStream {
                font: index,
                to_unicode: start..split,
                auxiliary: split..output.length,
                kind,
            });
        }
    }
    for usage in source.uses() {
        output.budget.step(1)?;
        let range = if let Some(text) = usage.actual_text() {
            let start = output.length;
            match text {
                BookV2FontUseText::Text(s) => font_encoding::utf16(output, s.chars(), true)?,
                BookV2FontUseText::Scalar(c) => {
                    font_encoding::utf16(output, std::iter::once(c), true)?
                }
            }
            Some(start..output.length)
        } else {
            None
        };
        if let Some(actual) = &mut actual {
            actual.push(range);
        }
    }
    Ok(())
}
pub struct BookV2FontStreamBuilder<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'z BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    budget: Budget,
}
impl<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontStreamBuilder<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        source: &'z BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.source().source().selection().display();
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let base = limits.base().get();
        let records = prior_records
            .max(source.record_charge())
            .checked_add(1)
            .ok_or(E::Records)?;
        let spool = prior_spool.max(source.spool_charge());
        let work = prior_work.max(source.work_steps());
        if records > base.max_fragments {
            return Err(E::Records);
        }
        if spool > base.max_spool_bytes {
            return Err(E::Spool);
        }
        if prior_output > base.max_output_bytes {
            return Err(E::Output);
        }
        if work > max_work {
            return Err(E::Work);
        }
        Ok(Self {
            source,
            budget: Budget {
                max_records: base.max_fragments,
                max_spool: base.max_spool_bytes,
                max_output: base.max_output_bytes,
                max_work,
                records,
                spool,
                output: prior_output,
                work,
            },
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
    ) -> Result<BookV2FontStreams<'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
        let source = self.source;
        let slots = source
            .fonts()
            .len()
            .checked_add(source.uses().len())
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let metadata = source
            .fonts()
            .len()
            .checked_mul(std::mem::size_of::<BookV2FontStream>())
            .and_then(|n| {
                n.checked_add(
                    source
                        .uses()
                        .len()
                        .checked_mul(std::mem::size_of::<Option<Range<usize>>>())?,
                )
            })
            .ok_or(E::Spool)?;
        let output_limit = self.budget.max_output - self.budget.output;
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
        encode(source, &mut counter, None, None)?;
        let length = counter.length;
        self.budget
            .reserve(slots, metadata.checked_add(length).ok_or(E::Spool)?, length)?;
        self.budget.step(1)?;
        let mut bytes = Vec::new();
        let mut fonts = Vec::new();
        let mut actual_text = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        fonts
            .try_reserve_exact(source.fonts().len())
            .map_err(|_| E::Allocation)?;
        actual_text
            .try_reserve_exact(source.uses().len())
            .map_err(|_| E::Allocation)?;
        let mut output = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        encode(
            source,
            &mut output,
            Some(&mut fonts),
            Some(&mut actual_text),
        )?;
        if output.length != length {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fingerprint = sha256(BOOK_V2_FONT_STREAMS_ALGORITHM.as_bytes());
        fingerprint = self.budget.fold(fingerprint, &source.fingerprint())?;
        fingerprint = self.budget.fold(fingerprint, &sha256(&bytes))?;
        for font in &fonts {
            let mut b = [0; 33];
            b[..8].copy_from_slice(&(font.font as u64).to_be_bytes());
            b[8..16].copy_from_slice(&(font.to_unicode.start as u64).to_be_bytes());
            b[16..24].copy_from_slice(&(font.auxiliary.start as u64).to_be_bytes());
            b[24..32].copy_from_slice(&(font.auxiliary.end as u64).to_be_bytes());
            b[32] = u8::from(font.kind == BookV2FontAuxiliaryKind::CffCidSet);
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        for range in &actual_text {
            let mut b = [0; 17];
            if let Some(range) = range {
                b[0] = 1;
                b[1..9].copy_from_slice(&(range.start as u64).to_be_bytes());
                b[9..].copy_from_slice(&(range.end as u64).to_be_bytes());
            }
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        Ok(BookV2FontStreams {
            source,
            fonts,
            actual_text,
            bytes,
            fingerprint,
            budget: self.budget,
        })
    }
}

#[path = "book_v2_font_objects.rs"]
mod objects;
pub use objects::*;

#[path = "book_v2_text_commands.rs"]
mod text_commands;
pub use text_commands::*;

#[path = "book_v2_vector_programs.rs"]
mod vector_programs;
pub use vector_programs::*;

#[path = "book_v2_image_objects.rs"]
mod image_objects;
pub use image_objects::*;

#[path = "book_v2_image_commands.rs"]
mod image_commands;
pub use image_commands::*;

#[path = "book_v2_marked_scopes.rs"]
mod marked_scopes;
pub use marked_scopes::*;

#[path = "book_v2_marked_content.rs"]
mod marked_content;
pub use marked_content::*;

#[path = "book_v2_source_structure.rs"]
mod source_structure;
pub use source_structure::*;
