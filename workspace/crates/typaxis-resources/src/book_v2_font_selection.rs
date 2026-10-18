//! Source-issued glyph selection for both TrueType and CFF /2. Subsetting and
//! CID/extraction encoding consume this selection in a later resource stage.
use typaxis_core::{sha256, M4EffectiveResourceLimits};
use typaxis_display_list::book_v2::{
    BookV2BodyDisplay, BookV2BodyPaintIndex, BookV2ColumnDisplay, BookV2ColumnPaintIndex,
    BookV2FontUseData, BookV2ResourceDisplay,
};
use typaxis_font::OriginalGlyphId;
use typaxis_resource_admission::AdmittedProductionFontInstanceV3;
#[path = "book_v2_font_closures.rs"]
mod closures;
pub use closures::*;
#[path = "book_v2_image_selection.rs"]
mod images;
pub use images::*;
#[path = "book_v2_font_command_session.rs"]
mod command_session;
use command_session::BookV2FontCffSession;
pub use command_session::{BookV2CommandFontBuilder, BookV2FontCommandSession};

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

pub struct BookV2SelectedFontUseData<'v, 'a, P: Copy> {
    paint: usize,
    usage: BookV2FontUseData<'v, 'a, P>,
}
impl<'v, 'a, P: Copy> BookV2SelectedFontUseData<'v, 'a, P> {
    pub fn paint_index(&self) -> usize {
        self.paint
    }
    pub fn usage(&self) -> &BookV2FontUseData<'v, 'a, P> {
        &self.usage
    }
}
pub type BookV2SelectedFontUse<'v, 'a> = BookV2SelectedFontUseData<'v, 'a, BookV2BodyPaintIndex>;
pub type BookV2ColumnSelectedFontUse<'v, 'a> =
    BookV2SelectedFontUseData<'v, 'a, BookV2ColumnPaintIndex>;
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
pub struct BookV2FontSelectionData<
    'v,
    'a,
    D: BookV2ResourceDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    display: &'v D,
    uses: Vec<BookV2SelectedFontUseData<'v, 'a, P>>,
    fonts: Vec<BookV2SelectedFont<'a>>,
    glyphs: Vec<FontGlyph<'a>>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'v, 'a, D: BookV2ResourceDisplay<'a>, P: Copy> BookV2FontSelectionData<'v, 'a, D, P> {
    pub fn display(&self) -> &'v D {
        self.display
    }
    pub fn uses(&self) -> &[BookV2SelectedFontUseData<'v, 'a, P>] {
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
pub struct BookV2FontSelectionBuilderData<'v, 'a, D: BookV2ResourceDisplay<'a>> {
    display: &'v D,
    cff_session: Option<BookV2FontCffSession<'v>>,
    maximum_records: u64,
    maximum_spool: u64,
    maximum_work: u64,
    records: u64,
    spool: u64,
    work: u64,
    lifetime: std::marker::PhantomData<&'a ()>,
}
pub type BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> = BookV2FontSelectionData<
    'v,
    'a,
    BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2BodyPaintIndex,
>;
/// Selected column uses cannot authorize the body resource chain.
///
/// ```compile_fail
/// use typaxis_resources::book_v2::{BookV2FontSelection, BookV2ColumnFontSelection};
/// fn body<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(selection: BookV2ColumnFontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2FontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> { selection }
/// ```
pub type BookV2ColumnFontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> = BookV2FontSelectionData<
    'v,
    'a,
    BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2ColumnPaintIndex,
>;
pub type BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2FontSelectionBuilderData<'v, 'a, BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>>;
/// The builder retains the same authentic column display for every stage.
///
/// ```compile_fail
/// use typaxis_resources::book_v2::{BookV2FontSelectionBuilder, BookV2ColumnFontSelectionBuilder};
/// fn body<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(builder: BookV2ColumnFontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> { builder }
/// ```
pub type BookV2ColumnFontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2FontSelectionBuilderData<'v, 'a, BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>>;
impl<'v, 'a, D: BookV2ResourceDisplay<'a>> BookV2FontSelectionBuilderData<'v, 'a, D> {
    pub fn new(
        display: &'v D,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        Self::new_counted(
            display,
            limits,
            maximum_work,
            prior_records,
            prior_spool,
            prior_work,
            &mut 0,
            &mut 0,
            &mut 0,
        )
    }
    /// Preserve accepted constructor prefixes when a later cap prevents a builder.
    #[allow(clippy::too_many_arguments)]
    pub fn new_counted(
        display: &'v D,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, E> {
        let records = prior_records.max(display.record_charge());
        let spool = prior_spool.max(display.source_spool_charge());
        let work = prior_work.max(display.work_steps());
        *observed_records = records;
        *observed_spool = spool;
        *observed_work = work;
        display
            .verify_resource_selection(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let records = records
            .checked_add(1)
            .filter(|n| *n <= limits.base().get().max_fragments)
            .ok_or(E::Records)?;
        *observed_records = records;
        if spool > limits.base().get().max_spool_bytes {
            return Err(E::Spool);
        }
        if work > maximum_work {
            return Err(E::Work);
        }
        Ok(Self {
            display,
            cff_session: Some(BookV2FontCffSession::Owned(
                typaxis_font::Cff1SubsetSessionV2::new(limits),
            )),
            maximum_records: limits.base().get().max_fragments,
            maximum_spool: limits.base().get().max_spool_bytes,
            maximum_work,
            records,
            spool,
            work,
            lifetime: std::marker::PhantomData,
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
    /// Reservation units for the private glyph and SFNT table buffers, in
    /// bytes. These diagnostics grant no resource construction authority.
    pub fn reservation_sizes(&self) -> [usize; 2] {
        [
            std::mem::size_of::<FontGlyph<'a>>(),
            std::mem::size_of::<crate::SfntRewriteTable>(),
        ]
    }

    /// Start the next resource stage with the same aggregate CFF session.
    /// The accepted constructor record remains charged on later cap errors.
    pub fn next_stage_counted(
        self,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, E> {
        let prior = [self.records, self.spool, self.work];
        self.next_stage_after_counted(
            prior[0],
            prior[1],
            prior[2],
            observed_records,
            observed_spool,
            observed_work,
        )
    }
    /// Carry an accepted downstream prefix while moving the same CFF session.
    /// The caller must bind the downstream owner to this authentic display.
    #[allow(clippy::too_many_arguments)]
    pub fn next_stage_after_counted(
        mut self,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, E> {
        self.records = self.records.max(prior_records);
        self.spool = self.spool.max(prior_spool);
        self.work = self.work.max(prior_work);
        *observed_records = self.records;
        *observed_spool = self.spool;
        *observed_work = self.work;
        self.display
            .verify_resource_selection(
                self.display.admitted(),
                self.display.admitted().effective_limits(),
            )
            .map_err(|_| E::Identity)?;
        let records = self
            .records
            .checked_add(1)
            .filter(|n| *n <= self.maximum_records)
            .ok_or(E::Records)?;
        *observed_records = records;
        if self.spool > self.maximum_spool {
            return Err(E::Spool);
        }
        if self.work > self.maximum_work {
            return Err(E::Work);
        }
        self.records = records;
        Ok(self)
    }
    /// Start the next stage while retaining a command session loan in this
    /// exact builder. Accepted constructor history survives a later refusal.
    #[allow(clippy::too_many_arguments)]
    pub fn next_stage_after_in_place_counted(
        &mut self,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
    ) -> Result<(), E> {
        self.records = self.records.max(prior_records);
        self.spool = self.spool.max(prior_spool);
        self.work = self.work.max(prior_work);
        *observed_records = self.records;
        *observed_spool = self.spool;
        *observed_work = self.work;
        self.display
            .verify_resource_selection(
                self.display.admitted(),
                self.display.admitted().effective_limits(),
            )
            .map_err(|_| E::Identity)?;
        let records = self
            .records
            .checked_add(1)
            .filter(|n| *n <= self.maximum_records)
            .ok_or(E::Records)?;
        // The loan remains usable after an error, so retain this accepted
        // reservation in both the builder and caller observation immediately.
        self.records = records;
        *observed_records = records;
        if self.spool > self.maximum_spool {
            return Err(E::Spool);
        }
        if self.work > self.maximum_work {
            return Err(E::Work);
        }
        Ok(())
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
    pub fn build(&mut self) -> Result<BookV2FontSelectionData<'v, 'a, D>, E> {
        let display = self.display;
        let mut count = 0usize;
        let mut glyph_count = 0usize;
        for paint in 0..display.paint_count() {
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
        let font_capacity = glyph_count.min(display.original_font_instance_count());
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
                    .checked_mul(std::mem::size_of::<
                        BookV2SelectedFontUseData<'v, 'a, D::Paint>,
                    >())
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
        let table = display.original_font_instance_fingerprint();
        for paint in 0..display.paint_count() {
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
                uses.push(BookV2SelectedFontUseData { paint, usage });
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
        Ok(BookV2FontSelectionData {
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

/// A scoped resource continuation keeps the original builder with its owner.
/// Resource results borrow their authentic source, rather than the callback loan.
///
/// ```
/// use typaxis_resources::book_v2::{BookV2ColumnFontSelectionBuilder as Builder, BookV2ColumnFontSelection as Selection, BookV2FontSelectionError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn build<'a>(builder: &mut B<'a>) -> Result<Selection<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>, BookV2FontSelectionError> {
///     builder.with_continuation(|loan| {
///         let _ = (loan.record_charge(), loan.spool_charge(), loan.work_steps());
///         loan.build()
///     })
/// }
/// ```
/// A fresh raw builder cannot replace the continuation.
///
/// ```compile_fail
/// use typaxis_resources::book_v2::BookV2ColumnFontSelectionBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn replace<'a>(builder: &mut B<'a>, fresh: B<'a>) {
///     builder.with_continuation(|loan| *loan = fresh);
/// }
/// ```
/// The raw builder is private.
///
/// ```compile_fail
/// use typaxis_resources::book_v2::BookV2ColumnFontSelectionBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn raw<'a>(builder: &mut B<'a>) {
///     builder.with_continuation(|loan| { let _ = &mut loan.builder; });
/// }
/// ```
/// The continuation cannot escape its fresh loan.
///
/// ```compile_fail
/// use typaxis_resources::book_v2::{BookV2ColumnFontSelectionBuilder as Builder, BookV2ColumnFontSelectionContinuation as Continuation};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn escape<'a>(builder: &'a mut B<'a>) -> &'a mut C<'a> {
///     builder.with_continuation(|loan| loan)
/// }
/// ```
/// Independent loans cannot exchange their continuations.
///
/// ```compile_fail
/// use typaxis_resources::book_v2::BookV2ColumnFontSelectionBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn swap<'a>(left: &mut B<'a>, right: &mut B<'a>) {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| std::mem::swap(outer, inner));
///     });
/// }
/// ```
/// Nested loans can inspect both original builders.
///
/// ```
/// use typaxis_resources::book_v2::BookV2ColumnFontSelectionBuilder as Builder;
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn inspect<'a>(left: &mut B<'a>, right: &mut B<'a>) -> u64 {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| outer.record_charge() + inner.record_charge())
///     })
/// }
/// ```
pub struct BookV2FontSelectionContinuationData<'loan, 'v, 'a, D: BookV2ResourceDisplay<'a>> {
    builder: &'loan mut BookV2FontSelectionBuilderData<'v, 'a, D>,
    invariant: std::marker::PhantomData<fn(&'loan ()) -> &'loan ()>,
}
pub type BookV2ColumnFontSelectionContinuation<'loan, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2FontSelectionContinuationData<
        'loan,
        'v,
        'a,
        BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    >;
impl<'loan, 'v, 'a, D: BookV2ResourceDisplay<'a>>
    BookV2FontSelectionContinuationData<'loan, 'v, 'a, D>
{
    pub fn record_charge(&self) -> u64 {
        self.builder.record_charge()
    }
    pub fn spool_charge(&self) -> u64 {
        self.builder.spool_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.builder.work_steps()
    }
    pub fn reservation_sizes(&self) -> [usize; 2] {
        self.builder.reservation_sizes()
    }
    pub fn cff_program_statistics(&self) -> BookV2CffProgramStatistics {
        self.builder.cff_program_statistics()
    }
    pub fn build(&mut self) -> Result<BookV2FontSelectionData<'v, 'a, D>, E> {
        self.builder.build()
    }
    pub fn prepare_font_closures<'c>(
        &mut self,
        selection: &'c BookV2FontSelectionData<'v, 'a, D>,
    ) -> Result<BookV2FontClosuresData<'c, 'v, 'a, D>, BookV2FontClosureError> {
        self.builder.prepare_font_closures(selection)
    }
    pub fn prepare_cff_programs(
        &mut self,
        closures: &BookV2FontClosuresData<'_, 'v, 'a, D>,
    ) -> Result<BookV2CffProgramStatistics, BookV2CffProgramError> {
        self.builder.prepare_cff_programs(closures)
    }
    pub fn write_font_programs<'x, 'c>(
        &mut self,
        closures: &'x BookV2FontClosuresData<'c, 'v, 'a, D>,
    ) -> Result<BookV2FontProgramsData<'x, 'c, 'v, 'a, D>, BookV2FontProgramsError> {
        self.builder.write_font_programs(closures)
    }
    pub fn plan_cids<'y, 'x, 'c>(
        &mut self,
        programs: &'y BookV2FontProgramsData<'x, 'c, 'v, 'a, D>,
    ) -> Result<BookV2CidPlansData<'y, 'x, 'c, 'v, 'a, D>, BookV2CidError> {
        self.builder.plan_cids(programs)
    }
    pub fn select_images(&mut self) -> Result<BookV2ImageSelectionData<'v, 'a, D>, E> {
        self.builder.select_images()
    }
    pub fn write_raster_programs<'i>(
        &mut self,
        selection: &'i BookV2ImageSelectionData<'v, 'a, D>,
    ) -> Result<BookV2RasterProgramsData<'i, 'v, 'a, D>, BookV2RasterError> {
        self.builder.write_raster_programs(selection)
    }
}
impl<'v, 'a, D: BookV2ResourceDisplay<'a>> BookV2FontSelectionBuilderData<'v, 'a, D> {
    /// Loan resource operations without transferring or replacing this builder.
    pub fn with_continuation<R>(
        &mut self,
        inspect: impl for<'loan> FnOnce(&mut BookV2FontSelectionContinuationData<'loan, 'v, 'a, D>) -> R,
    ) -> R {
        let mut continuation = BookV2FontSelectionContinuationData {
            builder: self,
            invariant: std::marker::PhantomData,
        };
        inspect(&mut continuation)
    }
}
