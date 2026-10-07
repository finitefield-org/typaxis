//! Source-issued glyph selection for both TrueType and CFF /2. Subsetting and
//! CID/extraction encoding consume this selection in a later resource stage.
use typaxis_core::{sha256, M4EffectiveResourceLimits};
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2FontUse};
use typaxis_font::OriginalGlyphId;
use typaxis_resource_admission::AdmittedProductionFontInstanceV3;
#[path = "book_v2_font_closures.rs"]
mod closures;
pub use closures::*;
#[path = "book_v2_image_selection.rs"]
mod images;
pub use images::*;

pub const BOOK_V2_FONT_SELECTION_ALGORITHM: &str = "typaxis.book-2-font-selection/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontSelectionError {
    Identity,
    Records,
    Spool,
    Work,
    Allocation,
}
impl std::fmt::Display for BookV2FontSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 font selection: {self:?}")
    }
}
impl std::error::Error for BookV2FontSelectionError {}
use BookV2FontSelectionError as E;

pub struct BookV2SelectedFontUse<'v, 'a> {
    paint: usize,
    usage: BookV2FontUse<'v, 'a>,
}
impl<'v, 'a> BookV2SelectedFontUse<'v, 'a> {
    pub fn paint_index(&self) -> usize {
        self.paint
    }
    pub fn usage(&self) -> &BookV2FontUse<'v, 'a> {
        &self.usage
    }
}
#[derive(Clone, Copy)]
struct FontGlyph<'a> {
    instance: AdmittedProductionFontInstanceV3<'a>,
    gid: OriginalGlyphId,
}
impl FontGlyph<'_> {
    fn key(&self) -> (u32, OriginalGlyphId) {
        (self.instance.font_instance_id().get(), self.gid)
    }
}
pub struct BookV2SelectedFont<'a> {
    instance: AdmittedProductionFontInstanceV3<'a>,
    range: std::ops::Range<usize>,
}
impl<'a> BookV2SelectedFont<'a> {
    pub fn instance(&self) -> AdmittedProductionFontInstanceV3<'a> {
        self.instance
    }
}
pub struct BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    uses: Vec<BookV2SelectedFontUse<'v, 'a>>,
    fonts: Vec<BookV2SelectedFont<'a>>,
    glyphs: Vec<FontGlyph<'a>>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn display(&self) -> &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.display
    }
    pub fn uses(&self) -> &[BookV2SelectedFontUse<'v, 'a>] {
        &self.uses
    }
    pub fn fonts(&self) -> &[BookV2SelectedFont<'a>] {
        &self.fonts
    }
    pub fn glyphs(
        &self,
        font: usize,
    ) -> Option<impl ExactSizeIterator<Item = OriginalGlyphId> + '_> {
        Some(
            self.glyphs
                .get(self.fonts.get(font)?.range.clone())?
                .iter()
                .map(|g| g.gid),
        )
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
pub struct BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    cff_session: Option<typaxis_font::Cff1SubsetSessionV2>,
    maximum_records: u64,
    maximum_spool: u64,
    maximum_work: u64,
    records: u64,
    spool: u64,
    work: u64,
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        display
            .verify_resource_selection(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let records = prior_records
            .max(display.record_charge())
            .checked_add(1)
            .ok_or(E::Records)?;
        let spool = prior_spool.max(display.source().spool_charge());
        let work = prior_work.max(display.work_steps());
        if records > limits.base().get().max_fragments {
            return Err(E::Records);
        }
        if spool > limits.base().get().max_spool_bytes {
            return Err(E::Spool);
        }
        if work > maximum_work {
            return Err(E::Work);
        }
        Ok(Self {
            display,
            cff_session: Some(typaxis_font::Cff1SubsetSessionV2::new(limits)),
            maximum_records: limits.base().get().max_fragments,
            maximum_spool: limits.base().get().max_spool_bytes,
            maximum_work,
            records,
            spool,
            work,
        })
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
    fn step(&mut self) -> Result<(), E> {
        self.work = self
            .work
            .checked_add(1)
            .filter(|n| *n <= self.maximum_work)
            .ok_or(E::Work)?;
        Ok(())
    }
    fn fold(&mut self, prior: [u8; 32], value: &[u8]) -> Result<[u8; 32], E> {
        for _ in 0..(32 + value.len()).div_ceil(64) {
            self.step()?;
        }
        let mut bytes = [0; 136];
        bytes[..32].copy_from_slice(&prior);
        bytes[32..32 + value.len()].copy_from_slice(value);
        Ok(sha256(&bytes[..32 + value.len()]))
    }
    pub fn build(&mut self) -> Result<BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
        let display = self.display;
        let mut count = 0usize;
        let mut glyph_count = 0usize;
        for paint in 0..display.paints().len() {
            self.step()?;
            for slot in 0..display.font_slot_count(paint).ok_or(E::Identity)? {
                self.step()?;
                if let Some(usage) = display.font_use(paint, slot).map_err(|_| E::Identity)? {
                    count = count.checked_add(1).ok_or(E::Records)?;
                    glyph_count = glyph_count
                        .checked_add(usage.glyphs().len())
                        .ok_or(E::Records)?;
                }
            }
        }
        let instances = display
            .source()
            .source()
            .flow()
            .lines()
            .prepared()
            .shaped()
            .font_instances();
        let font_capacity = glyph_count.min(instances.len());
        // Fonts are bounded by the actual instance table, not the number of
        // repeated glyphs. Deduplication does not refund reserved glyph slots.
        let required = glyph_count
            .checked_add(font_capacity)
            .and_then(|n| n.checked_add(count))
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let bytes = glyph_count
            .checked_mul(std::mem::size_of::<FontGlyph<'a>>())
            .and_then(|n| {
                font_capacity
                    .checked_mul(std::mem::size_of::<BookV2SelectedFont<'a>>())
                    .and_then(|f| n.checked_add(f))
            })
            .and_then(|n| {
                count
                    .checked_mul(std::mem::size_of::<BookV2SelectedFontUse<'v, 'a>>())
                    .and_then(|u| n.checked_add(u))
            })
            .ok_or(E::Spool)?;
        let records = self
            .records
            .checked_add(required as u64)
            .filter(|n| *n <= self.maximum_records)
            .ok_or(E::Records)?;
        let spool = self
            .spool
            .checked_add(bytes as u64)
            .filter(|n| *n <= self.maximum_spool)
            .ok_or(E::Spool)?;
        self.records = records;
        self.spool = spool;
        let mut uses = Vec::new();
        let mut glyphs = Vec::new();
        let mut fonts: Vec<BookV2SelectedFont<'a>> = Vec::new();
        uses.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        glyphs
            .try_reserve_exact(glyph_count)
            .map_err(|_| E::Allocation)?;
        fonts
            .try_reserve_exact(font_capacity)
            .map_err(|_| E::Allocation)?;
        self.step()?;
        let mut fp = sha256(BOOK_V2_FONT_SELECTION_ALGORITHM.as_bytes());
        fp = self.fold(fp, &display.fingerprint())?;
        fp = self.fold(fp, &display.admitted().fingerprint())?;
        let table = display
            .source()
            .source()
            .flow()
            .lines()
            .prepared()
            .shaped()
            .font_instances()
            .fingerprint();
        for paint in 0..display.paints().len() {
            self.step()?;
            for slot in 0..display.font_slot_count(paint).ok_or(E::Identity)? {
                self.step()?;
                let Some(usage) = display.font_use(paint, slot).map_err(|_| E::Identity)? else {
                    continue;
                };
                if usage.instance().table_fingerprint() != table {
                    return Err(E::Identity);
                }
                for index in 0..usage.glyphs().len() {
                    self.step()?;
                    let gid = usage.glyphs().get(index).ok_or(E::Identity)?;
                    if gid.get() == 0 {
                        return Err(E::Identity);
                    }
                    glyphs.push(FontGlyph {
                        instance: usage.instance(),
                        gid,
                    });
                }
                let mut meta = [0; 20];
                meta[..8].copy_from_slice(&(paint as u64).to_be_bytes());
                meta[8..16].copy_from_slice(&(slot as u64).to_be_bytes());
                meta[16..]
                    .copy_from_slice(&usage.instance().font_instance_id().get().to_be_bytes());
                fp = self.fold(fp, &meta)?;
                uses.push(BookV2SelectedFontUse { paint, usage });
            }
        }
        // Fallible heapsort bounds every comparison and swap through work;
        // exhausted work stops immediately, including partially sorted state.
        self.sort_by_key(&mut glyphs, 1, |g| g.key())?;
        let mut retained = 0usize;
        for index in 0..glyphs.len() {
            self.step()?;
            let glyph = glyphs[index];
            if retained > 0 && glyphs[retained - 1].key() == glyph.key() {
                continue;
            }
            if fonts
                .last()
                .is_none_or(|f| f.instance.font_instance_id() != glyph.instance.font_instance_id())
            {
                if fonts.len() >= font_capacity {
                    return Err(E::Identity);
                }
                fonts.push(BookV2SelectedFont {
                    instance: glyph.instance,
                    range: retained..retained,
                });
                fp = self.fold(fp, &glyph.instance.font().content_hash())?;
                fp = self.fold(fp, &glyph.instance.table_fingerprint())?;
                fp = self.fold(fp, &glyph.instance.font_instance_id().get().to_be_bytes())?;
            }
            glyphs[retained] = glyph;
            retained += 1;
            fonts.last_mut().ok_or(E::Identity)?.range.end = retained;
            fp = self.fold(fp, &glyph.gid.get().to_be_bytes())?;
        }
        glyphs.truncate(retained);
        Ok(BookV2FontSelection {
            display,
            uses,
            fonts,
            glyphs,
            fingerprint: fp,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
    fn sort_by_key<T, K: Ord>(
        &mut self,
        values: &mut [T],
        key_work: usize,
        key: impl Fn(&T) -> K + Copy,
    ) -> Result<(), E> {
        let count = values.len();
        for start in (0..count / 2).rev() {
            self.sift(values, start, count, key_work, key)?;
        }
        for end in (1..count).rev() {
            self.step()?;
            values.swap(0, end);
            self.sift(values, 0, end, key_work, key)?;
        }
        Ok(())
    }
    fn sift<T, K: Ord>(
        &mut self,
        glyphs: &mut [T],
        mut root: usize,
        end: usize,
        key_work: usize,
        key: impl Fn(&T) -> K + Copy,
    ) -> Result<(), E> {
        while let Some(mut child) = root
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n < end)
        {
            self.step()?;
            if child + 1 < end {
                self.closure_charge(0, 0, key_work)?;
                if key(&glyphs[child]) < key(&glyphs[child + 1]) {
                    child += 1;
                }
            }
            self.closure_charge(0, 0, key_work)?;
            if key(&glyphs[root]) >= key(&glyphs[child]) {
                break;
            }
            self.step()?;
            glyphs.swap(root, child);
            root = child;
        }
        Ok(())
    }
}
