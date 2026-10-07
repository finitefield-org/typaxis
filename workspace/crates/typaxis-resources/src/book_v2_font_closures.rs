//! Close every selected font before executing CFF programs or writing subsets.
use super::*;
#[path = "book_v2_cids.rs"]
mod cids;
pub use cids::*;
#[path = "book_v2_font_programs.rs"]
mod font_programs;
pub use font_programs::*;
#[path = "book_v2_cff_subsets.rs"]
mod cff_subsets;
pub use cff_subsets::*;
#[path = "book_v2_cff_programs.rs"]
mod cff_programs;
pub use cff_programs::*;
#[path = "book_v2_truetype_subsets.rs"]
mod truetype_subsets;
use std::collections::BTreeSet;
pub use truetype_subsets::*;
use typaxis_font::{Cff1GlyphClosureV2, Cff1SubsetSessionV2, SubsetGlyphId};
use typaxis_resource_admission::AdmittedProductionFontV3;

pub const BOOK_V2_FONT_CLOSURES_ALGORITHM: &str = "typaxis.book-2-font-closures/1";
#[derive(Debug)]
pub enum BookV2FontClosureError {
    SelectedGlyphLimit,
    Budget(BookV2FontSelectionError),
    TrueType(crate::ResourceError),
    Cff(typaxis_font::Cff1Error),
}
impl From<BookV2FontSelectionError> for BookV2FontClosureError {
    fn from(e: BookV2FontSelectionError) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2FontClosureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 font closure: {self:?}")
    }
}
impl std::error::Error for BookV2FontClosureError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontClosureKind {
    TrueType,
    Cff1V2,
}
enum Closure<'a> {
    TrueType(crate::PreparedTrueTypeSubset<'a>),
    Cff(Cff1GlyphClosureV2),
}
pub struct BookV2ClosedFont<'c, 'a> {
    source: &'c BookV2SelectedFont<'a>,
    closure: Closure<'a>,
    fingerprint: [u8; 32],
}
impl<'c, 'a> BookV2ClosedFont<'c, 'a> {
    pub fn source(&self) -> &'c BookV2SelectedFont<'a> {
        self.source
    }
    pub fn kind(&self) -> BookV2FontClosureKind {
        match self.closure {
            Closure::TrueType(_) => BookV2FontClosureKind::TrueType,
            Closure::Cff(_) => BookV2FontClosureKind::Cff1V2,
        }
    }
    pub fn glyphs(&self) -> BookV2ClosedGlyphs<'_> {
        BookV2ClosedGlyphs(match &self.closure {
            Closure::TrueType(t) => Glyphs::TrueType(t.closure.iter()),
            Closure::Cff(c) => Glyphs::Cff(c.source_gids().iter()),
        })
    }
    pub fn subset_gid(&self, source: OriginalGlyphId) -> Option<SubsetGlyphId> {
        match &self.closure {
            Closure::TrueType(t) => t.original_to_subset.get(&source).copied(),
            Closure::Cff(c) => c.subset_gid(source),
        }
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
enum Glyphs<'x> {
    TrueType(std::collections::btree_set::Iter<'x, u16>),
    Cff(std::slice::Iter<'x, OriginalGlyphId>),
}
pub struct BookV2ClosedGlyphs<'x>(Glyphs<'x>);
impl Iterator for BookV2ClosedGlyphs<'_> {
    type Item = OriginalGlyphId;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            Glyphs::TrueType(i) => i.next().copied().map(OriginalGlyphId::new),
            Glyphs::Cff(i) => i.next().copied(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            Glyphs::TrueType(i) => i.size_hint(),
            Glyphs::Cff(i) => i.size_hint(),
        }
    }
}
impl ExactSizeIterator for BookV2ClosedGlyphs<'_> {}
pub struct BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    selection: &'c BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    fonts: Vec<BookV2ClosedFont<'c, 'a>>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn selection(&self) -> &'c BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.selection
    }
    pub fn fonts(&self) -> &[BookV2ClosedFont<'c, 'a>] {
        &self.fonts
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
    pub(super) fn closure_charge(&mut self, records: usize, bytes: usize, work: usize) -> Result<(), E> {
        let records = self
            .records
            .checked_add(records as u64)
            .filter(|n| *n <= self.maximum_records)
            .ok_or(E::Records)?;
        let spool = self
            .spool
            .checked_add(bytes as u64)
            .filter(|n| *n <= self.maximum_spool)
            .ok_or(E::Spool)?;
        self.records = records;
        self.spool = spool;
        let work = u64::try_from(work).map_err(|_| E::Work)?;
        if work > self.maximum_work.saturating_sub(self.work) {
            self.work = self.maximum_work;
            return Err(E::Work);
        }
        self.work += work;
        Ok(())
    }
    pub fn prepare_font_closures<'c>(
        &mut self,
        selection: &'c BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2FontClosureError>
    {
        if !std::ptr::eq(self.display, selection.display()) {
            return Err(E::Identity.into());
        }
        self.records = self.records.max(selection.record_charge());
        self.spool = self.spool.max(selection.spool_charge());
        self.work = self.work.max(selection.work_steps());
        if self.records > self.maximum_records {
            return Err(E::Records.into());
        }
        if self.spool > self.maximum_spool {
            return Err(E::Spool.into());
        }
        if self.work > self.maximum_work {
            return Err(E::Work.into());
        }
        let count = selection.fonts().len();
        self.closure_charge(
            count.checked_add(1).ok_or(E::Records)?,
            count
                .checked_mul(std::mem::size_of::<BookV2ClosedFont<'c, 'a>>())
                .ok_or(E::Spool)?,
            1,
        )?;
        let mut fonts = Vec::new();
        fonts.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        let mut fingerprint = sha256(BOOK_V2_FONT_CLOSURES_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &selection.fingerprint())?;
        let limits = self.display.admitted().effective_limits();
        for (index, source) in selection.fonts().iter().enumerate() {
            let n = selection.glyphs(index).ok_or(E::Identity)?.len();
            if n > usize::from(limits.base().get().max_cids_per_font) {
                return Err(BookV2FontClosureError::SelectedGlyphLimit);
            }
            self.closure_charge(
                n.checked_add(1).ok_or(E::Records)?,
                n.checked_mul(128)
                    .and_then(|n| n.checked_add(512))
                    .ok_or(E::Spool)?,
                n.checked_mul(128)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(E::Work)?,
            )?;
            let requested = selection
                .glyphs(index)
                .ok_or(E::Identity)?
                .collect::<BTreeSet<_>>();
            let instance = source.instance();
            let closure = match instance.font() {
                AdmittedProductionFontV3::TrueType(font) => {
                    let mut failure = None;
                    let result = crate::prepare_truetype_subset_with_charge(
                        font.bytes(),
                        font.face_index(),
                        &requested,
                        &mut |records, bytes, work| {
                            self.closure_charge(records, bytes, work).map_err(|e| {
                                failure = Some(e);
                                crate::ResourceError::ResourceLimit
                            })
                        },
                    );
                    match result {
                        Ok(t) => Closure::TrueType(t),
                        Err(e) => {
                            return Err(match failure {
                                Some(e) => e.into(),
                                None => BookV2FontClosureError::TrueType(e),
                            })
                        }
                    }
                }
                AdmittedProductionFontV3::Cff1V2(font) => {
                    let len = n.checked_add(1).ok_or(E::Records)?;
                    let canonical = len
                        .checked_mul(6)
                        .and_then(|n| n.checked_add(512))
                        .ok_or(E::Spool)?;
                    self.closure_charge(
                        len.checked_add(1).ok_or(E::Records)?,
                        canonical
                            .checked_mul(2)
                            .and_then(|n| n.checked_add(len.checked_mul(2)?))
                            .ok_or(E::Spool)?,
                        len.checked_mul(16)
                            .and_then(|n| n.checked_add(canonical.div_ceil(64)))
                            .ok_or(E::Work)?,
                    )?;
                    Closure::Cff(
                        Cff1SubsetSessionV2::close_instance_selection(
                            font.admission(),
                            instance.font().font_face_id(),
                            instance.font_instance_id(),
                            &requested,
                            limits.base().get().max_cids_per_font,
                        )
                        .map_err(BookV2FontClosureError::Cff)?,
                    )
                }
            };
            let mut font = BookV2ClosedFont {
                source,
                closure,
                fingerprint: [0; 32],
            };
            let mut fp = self.fold(selection.fingerprint(), &instance.font().content_hash())?;
            fp = self.fold(fp, &instance.font_instance_id().get().to_be_bytes())?;
            fp = self.fold(
                fp,
                &[u8::from(font.kind() == BookV2FontClosureKind::Cff1V2)],
            )?;
            let lookup = match font.kind() {
                BookV2FontClosureKind::TrueType => 128,
                BookV2FontClosureKind::Cff1V2 => {
                    (usize::BITS - font.glyphs().len().leading_zeros()) as usize
                }
            };
            for (dense, glyph) in font.glyphs().enumerate() {
                self.closure_charge(0, 0, lookup + 1)?;
                let dense = u16::try_from(dense).map_err(|_| E::Records)?;
                if font.subset_gid(glyph).map(|g| g.get()) != Some(dense) {
                    return Err(E::Identity.into());
                }
                let mut bytes = [0; 4];
                bytes[..2].copy_from_slice(&glyph.get().to_be_bytes());
                bytes[2..].copy_from_slice(&dense.to_be_bytes());
                fp = self.fold(fp, &bytes)?;
            }
            font.fingerprint = fp;
            fingerprint = self.fold(fingerprint, &fp)?;
            fonts.push(font);
        }
        Ok(BookV2FontClosures {
            selection,
            fonts,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
}
