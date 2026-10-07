//! CID selection and occurrence-local extraction for real embedded programs.
use super::*;
use typaxis_display_list::book_v2::BookV2FontUseText;
use typaxis_font::{Cid, UnicodeScalar};

pub const BOOK_V2_CIDS_ALGORITHM: &str = "typaxis.book-2-cids/1";
#[derive(Debug)]
pub enum BookV2CidError {
    Budget(BookV2FontSelectionError),
    CidLimit,
    InvalidFontMapping,
}
impl From<E> for BookV2CidError {
    fn from(e: E) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2CidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 CID plan: {self:?}")
    }
}
impl std::error::Error for BookV2CidError {}
#[derive(Clone, Copy, Debug)]
pub struct BookV2CidBinding {
    original: OriginalGlyphId,
    cid: Cid,
    subset: SubsetGlyphId,
    width_1000: u32,
    // None = unseen, Some(None) = conflicting single-scalar observations.
    choice: Option<Option<UnicodeScalar>>,
}
impl BookV2CidBinding {
    pub fn original_gid(&self) -> OriginalGlyphId {
        self.original
    }
    pub fn cid(&self) -> Cid {
        self.cid
    }
    pub fn subset_gid(&self) -> SubsetGlyphId {
        self.subset
    }
    pub fn width_1000(&self) -> u32 {
        self.width_1000
    }
    pub fn unicode(&self) -> Option<char> {
        self.choice.flatten().map(|s| s.get())
    }
    fn observe(&mut self, scalar: char) {
        let scalar = UnicodeScalar::new(scalar);
        self.choice = Some(match self.choice {
            None => Some(scalar),
            Some(current) => crate::staging_text::merge_single_glyph_unicode(current, scalar),
        });
    }
}
pub struct BookV2CidFont<'y, 'x, 'c, 'a> {
    source: &'y BookV2FontProgram<'x, 'c, 'a>,
    range: std::ops::Range<usize>,
}
impl<'y, 'x, 'c, 'a> BookV2CidFont<'y, 'x, 'c, 'a> {
    pub fn source(&self) -> &'y BookV2FontProgram<'x, 'c, 'a> {
        self.source
    }
    pub fn font_instance_id(&self) -> typaxis_core::FontInstanceId {
        self.source.source().source().instance().font_instance_id()
    }
}
pub struct BookV2CidUse<'y, 'v, 'a> {
    source: &'y BookV2SelectedFontUse<'v, 'a>,
    font: usize,
    range: std::ops::Range<usize>,
    actual_text: bool,
}
impl<'y, 'v, 'a> BookV2CidUse<'y, 'v, 'a> {
    pub fn source(&self) -> &'y BookV2SelectedFontUse<'v, 'a> {
        self.source
    }
    pub fn font_index(&self) -> usize {
        self.font
    }
    pub fn requires_actual_text(&self) -> bool {
        self.actual_text
    }
    pub fn actual_text(&self) -> Option<BookV2FontUseText<'v>> {
        self.actual_text.then(|| self.source.usage().text())
    }
}
pub struct BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'y BookV2FontPrograms<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    fonts: Vec<BookV2CidFont<'y, 'x, 'c, 'a>>,
    bindings: Vec<BookV2CidBinding>,
    uses: Vec<BookV2CidUse<'y, 'v, 'a>>,
    cids: Vec<Cid>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'y BookV2FontPrograms<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn fonts(&self) -> &[BookV2CidFont<'y, 'x, 'c, 'a>] {
        &self.fonts
    }
    pub fn bindings(&self, font: usize) -> Option<&[BookV2CidBinding]> {
        self.bindings.get(self.fonts.get(font)?.range.clone())
    }
    pub fn uses(&self) -> &[BookV2CidUse<'y, 'v, 'a>] {
        &self.uses
    }
    pub fn cids(&self, usage: usize) -> Option<&[Cid]> {
        self.cids.get(self.uses.get(usage)?.range.clone())
    }
    pub fn usage_index(&self, paint: usize, slot: usize) -> Option<usize> {
        self.uses
            .binary_search_by_key(&(paint, slot), |u| {
                (u.source.paint_index(), u.source.usage().slot())
            })
            .ok()
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
enum Scalars<'a> {
    Text(std::str::Chars<'a>),
    Scalar(Option<char>),
}
impl Iterator for Scalars<'_> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        match self {
            Self::Text(s) => s.next(),
            Self::Scalar(c) => c.take(),
        }
    }
}
fn scalars(text: BookV2FontUseText<'_>) -> Scalars<'_> {
    match text {
        BookV2FontUseText::Text(s) => Scalars::Text(s.chars()),
        BookV2FontUseText::Scalar(c) => Scalars::Scalar(Some(c)),
    }
}
fn lookup_work(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as usize + 1
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn plan_cids<'y, 'x, 'c>(
        &mut self,
        programs: &'y BookV2FontPrograms<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2CidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2CidError>
    {
        let selection = programs.source().selection();
        if !std::ptr::eq(self.display, selection.display()) {
            return Err(E::Identity.into());
        }
        self.records = self.records.max(programs.record_charge());
        self.spool = self.spool.max(programs.spool_charge());
        self.work = self.work.max(programs.work_steps());
        if self.records > self.maximum_records {
            return Err(E::Records.into());
        }
        if self.spool > self.maximum_spool {
            return Err(E::Spool.into());
        }
        if self.work > self.maximum_work {
            return Err(E::Work.into());
        }
        let mut binding_count = 0usize;
        for (index, program) in programs.fonts().iter().enumerate() {
            self.step()?;
            if !std::ptr::eq(
                program.source().source(),
                selection.fonts().get(index).ok_or(E::Identity)?,
            ) {
                return Err(E::Identity.into());
            }
            let n = selection.glyphs(index).ok_or(E::Identity)?.len();
            if n > usize::from(
                self.display
                    .admitted()
                    .effective_limits()
                    .base()
                    .get()
                    .max_cids_per_font,
            ) {
                return Err(BookV2CidError::CidLimit);
            }
            binding_count = binding_count.checked_add(n).ok_or(E::Records)?;
        }
        if programs.fonts().len() != selection.fonts().len() {
            return Err(E::Identity.into());
        }
        let mut glyph_count = 0usize;
        for usage in selection.uses() {
            self.step()?;
            glyph_count = glyph_count
                .checked_add(usage.usage().glyphs().len())
                .ok_or(E::Records)?;
        }
        let nfonts = programs.fonts().len();
        let nuses = selection.uses().len();
        let records = nfonts
            .checked_add(binding_count)
            .and_then(|n| n.checked_add(nuses))
            .and_then(|n| n.checked_add(glyph_count))
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let bytes = nfonts
            .checked_mul(std::mem::size_of::<BookV2CidFont<'y, 'x, 'c, 'a>>())
            .and_then(|n| {
                n.checked_add(binding_count.checked_mul(std::mem::size_of::<BookV2CidBinding>())?)
            })
            .and_then(|n| {
                n.checked_add(nuses.checked_mul(std::mem::size_of::<BookV2CidUse<'y, 'v, 'a>>())?)
            })
            .and_then(|n| n.checked_add(glyph_count.checked_mul(std::mem::size_of::<Cid>())?))
            .ok_or(E::Spool)?;
        self.closure_charge(records, bytes, 1)?;
        let mut fonts = Vec::new();
        let mut bindings = Vec::new();
        let mut uses = Vec::new();
        let mut cids = Vec::new();
        fonts.try_reserve_exact(nfonts).map_err(|_| E::Allocation)?;
        bindings
            .try_reserve_exact(binding_count)
            .map_err(|_| E::Allocation)?;
        uses.try_reserve_exact(nuses).map_err(|_| E::Allocation)?;
        cids.try_reserve_exact(glyph_count)
            .map_err(|_| E::Allocation)?;
        for (index, source) in programs.fonts().iter().enumerate() {
            let start = bindings.len();
            let units = u64::from(
                source
                    .source()
                    .source()
                    .instance()
                    .font()
                    .metadata()
                    .units_per_em,
            );
            if units == 0 {
                return Err(BookV2CidError::InvalidFontMapping);
            }
            for (offset, original) in selection.glyphs(index).ok_or(E::Identity)?.enumerate() {
                self.closure_charge(0, 0, 256)?;
                let cid = u16::try_from(offset + 1)
                    .ok()
                    .and_then(Cid::new)
                    .ok_or(BookV2CidError::CidLimit)?;
                let subset = source
                    .subset_gid(original)
                    .ok_or(BookV2CidError::InvalidFontMapping)?;
                if original.get() == 0
                    || subset.get() == 0
                    || (source.kind() == BookV2FontClosureKind::Cff1V2 && subset.get() != cid.get())
                {
                    return Err(BookV2CidError::InvalidFontMapping);
                }
                let advance = u64::from(
                    source
                        .advance(original)
                        .ok_or(BookV2CidError::InvalidFontMapping)?,
                );
                let width_1000 = u32::try_from((advance * 1000 + units / 2) / units)
                    .map_err(|_| BookV2CidError::InvalidFontMapping)?;
                bindings.push(BookV2CidBinding {
                    original,
                    cid,
                    subset,
                    width_1000,
                    choice: None,
                });
            }
            fonts.push(BookV2CidFont {
                source,
                range: start..bindings.len(),
            });
        }
        // Only direct, single-scalar/single-glyph observations propose a
        // ToUnicode value. Complex occurrences keep their exact source text.
        for selected in selection.uses() {
            self.closure_charge(0, 0, 9)?;
            let usage = selected.usage();
            if usage.glyphs().len() != 1 {
                continue;
            }
            let mut text = scalars(usage.text());
            let Some(scalar) = text.next() else {
                continue;
            };
            if text.next().is_some() {
                continue;
            }
            self.closure_charge(0, 0, lookup_work(fonts.len()))?;
            let font = fonts
                .binary_search_by_key(&usage.instance().font_instance_id(), |f| {
                    f.font_instance_id()
                })
                .map_err(|_| E::Identity)?;
            let range = fonts[font].range.clone();
            self.closure_charge(0, 0, lookup_work(range.len()))?;
            let original = usage.glyphs().get(0).ok_or(E::Identity)?;
            let offset = bindings[range.clone()]
                .binary_search_by_key(&original, |b| b.original)
                .map_err(|_| BookV2CidError::InvalidFontMapping)?;
            bindings[range.start + offset].observe(scalar);
        }
        let mut fingerprint = sha256(BOOK_V2_CIDS_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &programs.fingerprint())?;
        for font in &fonts {
            fingerprint = self.fold(fingerprint, &font.font_instance_id().get().to_be_bytes())?;
            for binding in &bindings[font.range.clone()] {
                let mut encoded = [0; 15];
                encoded[..2].copy_from_slice(&binding.original.get().to_be_bytes());
                encoded[2..4].copy_from_slice(&binding.cid.get().to_be_bytes());
                encoded[4..6].copy_from_slice(&binding.subset.get().to_be_bytes());
                encoded[6..10].copy_from_slice(&binding.width_1000.to_be_bytes());
                if let Some(c) = binding.unicode() {
                    encoded[10] = 1;
                    encoded[11..].copy_from_slice(&(c as u32).to_be_bytes());
                }
                fingerprint = self.fold(fingerprint, &encoded)?;
            }
        }
        for (index, source) in selection.uses().iter().enumerate() {
            self.closure_charge(0, 0, lookup_work(fonts.len()))?;
            let usage = source.usage();
            let font = fonts
                .binary_search_by_key(&usage.instance().font_instance_id(), |f| {
                    f.font_instance_id()
                })
                .map_err(|_| E::Identity)?;
            let range = fonts[font].range.clone();
            let start = cids.len();
            let mut text = scalars(usage.text());
            let mut matches = true;
            for glyph in 0..usage.glyphs().len() {
                self.closure_charge(0, 0, lookup_work(range.len()) + 5)?;
                let original = usage.glyphs().get(glyph).ok_or(E::Identity)?;
                let offset = bindings[range.clone()]
                    .binary_search_by_key(&original, |b| b.original)
                    .map_err(|_| BookV2CidError::InvalidFontMapping)?;
                let binding = bindings[range.start + offset];
                cids.push(binding.cid);
                if let Some(unicode) = binding.unicode() {
                    matches &= text.next() == Some(unicode);
                }
                fingerprint = self.fold(fingerprint, &binding.cid.get().to_be_bytes())?;
            }
            self.closure_charge(0, 0, 4)?;
            matches &= text.next().is_none();
            let mut encoded = [0; 25];
            encoded[..8].copy_from_slice(&(index as u64).to_be_bytes());
            encoded[8..16].copy_from_slice(&(font as u64).to_be_bytes());
            encoded[16..24].copy_from_slice(&((cids.len() - start) as u64).to_be_bytes());
            encoded[24] = u8::from(!matches);
            fingerprint = self.fold(fingerprint, &encoded)?;
            uses.push(BookV2CidUse {
                source,
                font,
                range: start..cids.len(),
                actual_text: !matches,
            });
        }
        Ok(BookV2CidPlans {
            source: programs,
            fonts,
            bindings,
            uses,
            cids,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
}
