//! Metered counterpart of read-fonts 0.31.3's GPOS Intersect implementation.
//! Preserve lazy errors, iteration order and malformed-table behavior. Readers
//! return borrowed views; every variable-length loop is driven here.
use super::{Stage, State, READER_WORK, STEP_WORK};
use crate::{CffAdmissionBudgetV2, CffTableAdmissionBudgetFailureV2};
use read_fonts::{
    tables::{gpos::*, layout::*},
    types::{BigEndian, GlyphId16},
    ArrayOfOffsets, FontData, FontRead, ReadError,
};
#[derive(Debug)]
pub(super) enum Failure<E> {
    Native(ReadError),
    Budget(CffTableAdmissionBudgetFailureV2<E>),
}
type Result<T, E> = std::result::Result<T, Failure<E>>;
impl<E> From<CffTableAdmissionBudgetFailureV2<E>> for Failure<E> {
    fn from(e: CffTableAdmissionBudgetFailureV2<E>) -> Self {
        Self::Budget(e)
    }
}
impl<B: CffAdmissionBudgetV2> State<'_, B> {
    fn tick(&mut self) -> Result<(), B::Error> {
        self.charge(Stage::JstfVisit, None, STEP_WORK)
            .map_err(Failure::Budget)
    }
    fn reader(&mut self, data: Option<FontData<'_>>) -> Result<(), B::Error> {
        let at = data.and_then(|d| {
            let bytes = d.as_bytes();
            let n = (bytes.as_ptr() as usize).checked_sub(self.start)?;
            n.checked_add(bytes.len())
                .filter(|end| *end <= self.length)
                .map(|_| n)
        });
        self.charge(Stage::JstfReader, at, READER_WORK)
            .map_err(Failure::Budget)
    }
}
macro_rules! read {
    ($s:expr,$owner:expr,$value:expr) => {{
        $s.reader(Some($owner.offset_data()))?;
        $value.map_err(Failure::Native)?
    }};
}

/// The glyph universe is exactly the original 0..glyph_count IntSet. Temporary
/// sets have 65,536 bits, including class 65535; no heap is used by this module.
struct Bits {
    words: [u64; 1024],
    len: u32,
    first: Option<u16>,
    last: Option<u16>,
}
impl Bits {
    fn new<B: CffAdmissionBudgetV2>(s: &mut State<'_, B>) -> Result<Self, B::Error> {
        s.charge(Stage::JstfSetInit, None, 1024 * STEP_WORK)?;
        Ok(Self {
            words: [0; 1024],
            len: 0,
            first: None,
            last: None,
        })
    }
    fn insert(&mut self, n: u16) {
        let word = &mut self.words[n as usize / 64];
        let mask = 1u64 << (n % 64);
        if *word & mask == 0 {
            *word |= mask;
            self.len += 1;
            self.first = Some(self.first.map_or(n, |v| v.min(n)));
            self.last = Some(self.last.map_or(n, |v| v.max(n)));
        }
    }
}
#[derive(Clone, Copy)]
enum Set<'a> {
    Dense(u16),
    Bits(&'a Bits),
}
impl Set<'_> {
    fn len(self) -> u64 {
        match self {
            Self::Dense(n) => n as u64,
            Self::Bits(b) => b.len as u64,
        }
    }
    fn contains(self, g: u16) -> bool {
        match self {
            Self::Dense(n) => g < n,
            Self::Bits(b) => b.words[g as usize / 64] & (1u64 << (g % 64)) != 0,
        }
    }
    fn first(self) -> Option<u16> {
        match self {
            Self::Dense(n) => (n != 0).then_some(0),
            Self::Bits(b) => b.first,
        }
    }
    fn last(self) -> Option<u16> {
        match self {
            Self::Dense(n) => n.checked_sub(1),
            Self::Bits(b) => b.last,
        }
    }
    fn next<B: CffAdmissionBudgetV2>(
        self,
        after: Option<u16>,
        s: &mut State<'_, B>,
    ) -> Result<Option<u16>, B::Error> {
        s.tick()?;
        let start = after.map_or(0, |g| g as u32 + 1);
        match self {
            Self::Dense(n) => Ok((start < n as u32).then_some(start as u16)),
            Self::Bits(b) => {
                let mut index = start as usize / 64;
                if index == 1024 {
                    return Ok(None);
                }
                let mut bits = b.words[index] & (!0u64 << (start % 64));
                loop {
                    s.tick()?;
                    if bits != 0 {
                        return Ok(Some((index * 64 + bits.trailing_zeros() as usize) as u16));
                    }
                    index += 1;
                    if index == 1024 {
                        return Ok(None);
                    }
                    bits = b.words[index];
                }
            }
        }
    }
    fn range<B: CffAdmissionBudgetV2>(
        self,
        start: u16,
        end: u16,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        if start > end {
            s.tick()?;
            return Ok(false);
        }
        Ok(self
            .next(start.checked_sub(1), s)?
            .is_some_and(|g| g <= end))
    }
}

struct CoverageIter<'a> {
    table: CoverageTable<'a>,
    index: usize,
    next: Option<u32>,
}
impl<'a> CoverageIter<'a> {
    fn new(table: CoverageTable<'a>) -> Self {
        Self {
            table,
            index: 0,
            next: None,
        }
    }
    fn next<B: CffAdmissionBudgetV2>(
        &mut self,
        s: &mut State<'_, B>,
    ) -> Result<Option<u16>, B::Error> {
        match &self.table {
            CoverageTable::Format1(t) => {
                s.tick()?;
                let g = t.glyph_array().get(self.index).map(|g| g.get().to_u16());
                self.index += 1;
                Ok(g)
            }
            CoverageTable::Format2(t) => loop {
                s.tick()?;
                let Some(r) = t.range_records().get(self.index) else {
                    return Ok(None);
                };
                let next = self.next.unwrap_or(r.start_glyph_id().to_u16() as u32);
                let end = r.end_glyph_id().to_u16() as u32;
                if next <= end {
                    self.next = Some(next + 1);
                    return Ok(Some(next as u16));
                }
                self.index += 1;
                self.next = None;
            },
        }
    }
}
fn coverage_count(t: &CoverageTable<'_>) -> u32 {
    match t {
        CoverageTable::Format1(t) => t.glyph_count() as u32,
        CoverageTable::Format2(t) => t.range_count() as u32,
    }
}
fn coverage_get<B: CffAdmissionBudgetV2>(
    t: &CoverageTable<'_>,
    g: u16,
    s: &mut State<'_, B>,
) -> Result<Option<u16>, B::Error> {
    s.reader(Some(t.offset_data()))?;
    Ok(t.get(GlyphId16::new(g)))
}
fn coverage_intersects<B: CffAdmissionBudgetV2>(
    t: CoverageTable<'_>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    s.tick()?;
    let count = coverage_count(&t);
    let bits = 32 - count.leading_zeros();
    if count > (set.len() as u32) * bits {
        let mut previous = None;
        while let Some(g) = set.next(previous, s)? {
            previous = Some(g);
            if coverage_get(&t, g, s)?.is_some() {
                return Ok(true);
            }
        }
    } else {
        match t {
            CoverageTable::Format1(t) => {
                for g in t.glyph_array() {
                    s.tick()?;
                    if set.contains(g.get().to_u16()) {
                        return Ok(true);
                    }
                }
            }
            CoverageTable::Format2(t) => {
                for r in t.range_records() {
                    s.tick()?;
                    if set.range(r.start_glyph_id().to_u16(), r.end_glyph_id().to_u16(), s)? {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
fn coverage_set<B: CffAdmissionBudgetV2>(
    t: CoverageTable<'_>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<Bits, B::Error> {
    let mut out = Bits::new(s)?;
    s.tick()?;
    let count = coverage_count(&t);
    let bits = 32 - count.leading_zeros();
    if count > (set.len() as u32) * bits {
        let mut previous = None;
        while let Some(g) = set.next(previous, s)? {
            previous = Some(g);
            if coverage_get(&t, g, s)?.is_some() {
                out.insert(g);
            }
        }
    } else {
        match t {
            CoverageTable::Format1(t) => {
                for g in t.glyph_array() {
                    s.tick()?;
                    let g = g.get().to_u16();
                    if set.contains(g) {
                        out.insert(g);
                    }
                }
            }
            CoverageTable::Format2(t) => {
                let mut last = 0;
                for r in t.range_records() {
                    s.tick()?;
                    let start = r.start_glyph_id().to_u16();
                    if start < last {
                        break;
                    }
                    let end = r.end_glyph_id().to_u16();
                    last = end;
                    // The locked library inserts start even for a reversed record.
                    if set.contains(start) {
                        out.insert(start);
                    }
                    let mut previous = Some(start);
                    while let Some(g) = set.next(previous, s)? {
                        previous = Some(g);
                        if g > end {
                            break;
                        }
                        out.insert(g);
                    }
                }
            }
        }
    }
    Ok(out)
}
fn class_intersects<B: CffAdmissionBudgetV2>(
    t: ClassDef<'_>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    s.tick()?;
    match t {
        ClassDef::Format1(t) => {
            let count = t.glyph_count();
            if count == 0 {
                return Ok(false);
            }
            let start = t.start_glyph_id().to_u16();
            let end = start as u32 + count as u32;
            let values = t.class_value_array();
            if set.contains(start) && values[0] != 0 {
                return Ok(true);
            }
            // Deliberately preserve read-fonts 0.31.3's constant cursor. A finite
            // observed budget can stop a native non-terminating zero-class walk.
            while let Some(g) = set.next(Some(start), s)? {
                if g as u32 >= end {
                    break;
                }
                let Some(c) = values.get((g - start) as usize) else {
                    break;
                };
                if c.get() != 0 {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ClassDef::Format2(t) => {
            let count = t.class_range_count();
            let bits = 16 - count.leading_zeros();
            if count as u64 > set.len() * bits as u64 {
                let mut previous = None;
                while let Some(g) = set.next(previous, s)? {
                    previous = Some(g);
                    s.reader(Some(t.offset_data()))?;
                    if t.get(GlyphId16::new(g)) != 0 {
                        return Ok(true);
                    }
                }
            } else {
                for r in t.class_range_records() {
                    s.tick()?;
                    if set.range(r.start_glyph_id().to_u16(), r.end_glyph_id().to_u16(), s)?
                        && r.class() != 0
                    {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
    }
}
fn class_set<B: CffAdmissionBudgetV2>(
    t: ClassDef<'_>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<Bits, B::Error> {
    let mut out = Bits::new(s)?;
    s.tick()?;
    let Some(first) = set.first() else {
        return Ok(out);
    };
    match t {
        ClassDef::Format1(t) => {
            let start = t.start_glyph_id().to_u16();
            let end = start as u32 + t.glyph_count() as u32 - 1;
            if first < start || set.last().unwrap() as u32 > end {
                out.insert(0);
            }
            let values = t.class_value_array();
            if set.contains(start) {
                let Some(c) = values.first() else {
                    return Ok(out);
                };
                out.insert(c.get());
            }
            let mut previous = Some(start);
            while let Some(g) = set.next(previous, s)? {
                previous = Some(g);
                if g as u32 > end {
                    break;
                }
                let Some(c) = values.get((g - start) as usize) else {
                    break;
                };
                out.insert(c.get());
            }
        }
        ClassDef::Format2(t) => {
            let ranges = t.class_range_records();
            let Some(first_record) = ranges.first() else {
                out.insert(0);
                return Ok(out);
            };
            if first < first_record.start_glyph_id().to_u16() {
                out.insert(0);
            } else {
                let mut glyph = first_record.end_glyph_id().to_u16();
                for r in ranges.iter().skip(1) {
                    s.tick()?;
                    let Some(g) = set.next(Some(glyph), s)? else {
                        break;
                    };
                    if g < r.start_glyph_id().to_u16() {
                        out.insert(0);
                        break;
                    }
                    glyph = r.end_glyph_id().to_u16();
                }
                if set.next(Some(glyph), s)?.is_some() {
                    out.insert(0);
                }
            }
            let count = t.class_range_count();
            let bits = 16 - count.leading_zeros();
            if count as u64 > set.len() * bits as u64 {
                let mut previous = None;
                while let Some(g) = set.next(previous, s)? {
                    previous = Some(g);
                    s.reader(Some(t.offset_data()))?;
                    let c = t.get(GlyphId16::new(g));
                    if c != 0 {
                        out.insert(c);
                    }
                }
            } else {
                for r in ranges {
                    s.tick()?;
                    if set.range(r.start_glyph_id().to_u16(), r.end_glyph_id().to_u16(), s)? {
                        out.insert(r.class());
                    }
                }
            }
        }
    }
    Ok(out)
}

trait Walk<'a>: FontRead<'a> {
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error>;
}
fn subtables<'a, T: Walk<'a> + 'a, Ext: ExtensionLookup<'a, T> + 'a, B: CffAdmissionBudgetV2>(
    tables: Subtables<'a, T, Ext>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    for i in 0..tables.len() {
        s.reader(None)?;
        let table = tables.get(i).map_err(Failure::Native)?;
        if table.walk(set, s)? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn lookup<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyphs: u16,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    s.reader(Some(FontData::new(bytes)))?;
    let lookup = PositionLookup::read(FontData::new(bytes)).map_err(Failure::Native)?;
    s.reader(Some(FontData::new(bytes)))?;
    let tables = lookup.subtables().map_err(Failure::Native)?;
    let set = Set::Dense(glyphs);
    match tables {
        PositionSubtables::Single(t) => subtables(t, set, s),
        PositionSubtables::Pair(t) => subtables(t, set, s),
        PositionSubtables::Cursive(t) => subtables(t, set, s),
        PositionSubtables::MarkToBase(t) => subtables(t, set, s),
        PositionSubtables::MarkToLig(t) => subtables(t, set, s),
        PositionSubtables::MarkToMark(t) => subtables(t, set, s),
        PositionSubtables::Contextual(t) => subtables(t, set, s),
        PositionSubtables::ChainContextual(t) => subtables(t, set, s),
    }
}
impl<'a> Walk<'a> for SinglePos<'a> {
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        let coverage = match self {
            Self::Format1(t) => read!(s, t, t.coverage()),
            Self::Format2(t) => read!(s, t, t.coverage()),
        };
        coverage_intersects(coverage, set, s)
    }
}
macro_rules! coverage_walk {
    ($name:ident,$first:ident $(,$second:ident)?)=>{
        impl<'a> Walk<'a> for $name<'a> {
            fn walk<B:CffAdmissionBudgetV2>(&self,set:Set<'_>,s:&mut State<'_,B>)->Result<bool,B::Error> {
                let coverage=read!(s,self,self.$first());
                if !coverage_intersects(coverage,set,s)?{return Ok(false);}
                $(let coverage=read!(s,self,self.$second());
                  if !coverage_intersects(coverage,set,s)?{return Ok(false);})?
                Ok(true)
            }
        }
    };
}
coverage_walk!(CursivePosFormat1, coverage);
coverage_walk!(MarkBasePosFormat1, mark_coverage, base_coverage);
coverage_walk!(MarkLigPosFormat1, mark_coverage, ligature_coverage);
coverage_walk!(MarkMarkPosFormat1, mark1_coverage, mark2_coverage);
fn pair_set<B: CffAdmissionBudgetV2>(
    t: PairSet<'_>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    s.reader(Some(t.offset_data()))?;
    let mut iter = t.pair_value_records().iter();
    loop {
        s.reader(Some(t.offset_data()))?;
        let Some(record) = iter.next() else {
            break;
        };
        if set.contains(record.map_err(Failure::Native)?.second_glyph().to_u16()) {
            return Ok(true);
        }
    }
    Ok(false)
}
impl<'a> Walk<'a> for PairPos<'a> {
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        match self {
            Self::Format1(t) => {
                let coverage = read!(s, t, t.coverage());
                let pair_sets = t.pair_sets();
                let count = t.pair_set_count();
                let bits = 16 - count.leading_zeros();
                if count as u64 > set.len() * bits as u64 {
                    let mut previous = None;
                    while let Some(g) = set.next(previous, s)? {
                        previous = Some(g);
                        let Some(i) = coverage_get(&coverage, g, s)? else {
                            continue;
                        };
                        let table = read!(s, t, pair_sets.get(i as usize));
                        if pair_set(table, set, s)? {
                            return Ok(true);
                        }
                    }
                } else {
                    let mut coverage = CoverageIter::new(coverage);
                    let mut index = 0;
                    while let Some(g) = coverage.next(s)? {
                        if index >= pair_sets.len() {
                            break;
                        }
                        s.reader(Some(t.offset_data()))?;
                        let table = pair_sets.get(index);
                        index += 1;
                        if !set.contains(g) {
                            continue;
                        }
                        if pair_set(table.map_err(Failure::Native)?, set, s)? {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
            Self::Format2(t) => {
                let coverage = read!(s, t, t.coverage());
                if !coverage_intersects(coverage, set, s)? {
                    return Ok(false);
                }
                let class = read!(s, t, t.class_def2());
                class_intersects(class, set, s)
            }
        }
    }
}

fn glyph_sequence<B: CffAdmissionBudgetV2>(
    values: &[BigEndian<GlyphId16>],
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    for g in values {
        s.tick()?;
        if !set.contains(g.get().to_u16()) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn class_sequence<B: CffAdmissionBudgetV2>(
    values: &[BigEndian<u16>],
    set: &Bits,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    for c in values {
        s.tick()?;
        if !Set::Bits(set).contains(c.get()) {
            return Ok(false);
        }
    }
    Ok(true)
}
impl Context1<'_> {
    fn data(&self) -> FontData<'_> {
        match self {
            Self::Plain(t) => t.offset_data(),
            Self::Chain(t) => t.offset_data(),
        }
    }
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        s.reader(Some(self.data()))?;
        let coverage = self.coverage().map_err(Failure::Native)?;
        let mut coverage = CoverageIter::new(coverage);
        let mut rule_sets = self.rule_sets();
        while let Some(g) = coverage.next(s)? {
            s.reader(Some(self.data()))?;
            let Some(rs) = rule_sets.next() else {
                break;
            };
            if !set.contains(g) {
                continue;
            }
            let Some(rs) = rs else {
                continue;
            };
            let rs = rs.map_err(Failure::Native)?;
            let mut rules = rs.rules();
            loop {
                s.reader(None)?;
                let Some(rule) = rules.next() else {
                    break;
                };
                let rule = rule.map_err(Failure::Native)?;
                let hit = match rule {
                    Rule1::Plain(t) => glyph_sequence(t.input_sequence(), set, s)?,
                    Rule1::Chain(t) => {
                        glyph_sequence(t.backtrack_sequence(), set, s)?
                            && glyph_sequence(t.input_sequence(), set, s)?
                            && glyph_sequence(t.lookahead_sequence(), set, s)?
                    }
                };
                if hit {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
impl Context2<'_> {
    fn data(&self) -> FontData<'_> {
        match self {
            Self::Plain(t) => t.offset_data(),
            Self::Chain(t) => t.offset_data(),
        }
    }
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        s.reader(Some(self.data()))?;
        let coverage = self.coverage().map_err(Failure::Native)?;
        let retained = coverage_set(coverage, set, s)?;
        if retained.len == 0 {
            return Ok(false);
        }
        s.reader(Some(self.data()))?;
        let input_def = self.input_class_def().map_err(Failure::Native)?;
        let coverage_classes = class_set(input_def.clone(), Set::Bits(&retained), s)?;
        let input_classes = class_set(input_def, set, s)?;
        let backtrack_classes = match self {
            Self::Plain(_) => Bits::new(s)?,
            Self::Chain(t) => class_set(read!(s, t, t.backtrack_class_def()), set, s)?,
        };
        let lookahead_classes = match self {
            Self::Plain(_) => Bits::new(s)?,
            Self::Chain(t) => class_set(read!(s, t, t.lookahead_class_def()), set, s)?,
        };
        let mut rule_sets = self.rule_sets().enumerate();
        loop {
            s.reader(Some(self.data()))?;
            let Some((class, rs)) = rule_sets.next() else {
                break;
            };
            if !Set::Bits(&coverage_classes).contains(class as u16) {
                continue;
            }
            let Some(rs) = rs else {
                continue;
            };
            let rs = rs.map_err(Failure::Native)?;
            let mut rules = rs.rules();
            loop {
                s.reader(None)?;
                let Some(rule) = rules.next() else {
                    break;
                };
                let rule = rule.map_err(Failure::Native)?;
                let hit = match rule {
                    Rule2::Plain(t) => class_sequence(t.input_sequence(), &input_classes, s)?,
                    Rule2::Chain(t) => {
                        class_sequence(t.input_sequence(), &input_classes, s)?
                            && class_sequence(t.backtrack_sequence(), &backtrack_classes, s)?
                            && class_sequence(t.lookahead_sequence(), &lookahead_classes, s)?
                    }
                };
                if hit {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
fn coverages<'a, B: CffAdmissionBudgetV2>(
    tables: ArrayOfOffsets<'a, CoverageTable<'a>>,
    set: Set<'_>,
    s: &mut State<'_, B>,
) -> Result<bool, B::Error> {
    for i in 0..tables.len() {
        s.reader(None)?;
        let t = tables.get(i).map_err(Failure::Native)?;
        if !coverage_intersects(t, set, s)? {
            return Ok(false);
        }
    }
    Ok(true)
}
impl<'a> Walk<'a> for SequenceContext<'a> {
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        s.reader(Some(self.offset_data()))?;
        match self {
            Self::Format1(t) => Context1::Plain(t.clone()).walk(set, s),
            Self::Format2(t) => Context2::Plain(t.clone()).walk(set, s),
            Self::Format3(t) => coverages(t.coverages(), set, s),
        }
    }
}
impl<'a> Walk<'a> for ChainedSequenceContext<'a> {
    fn walk<B: CffAdmissionBudgetV2>(
        &self,
        set: Set<'_>,
        s: &mut State<'_, B>,
    ) -> Result<bool, B::Error> {
        s.reader(Some(self.offset_data()))?;
        match self {
            Self::Format1(t) => Context1::Chain(t.clone()).walk(set, s),
            Self::Format2(t) => Context2::Chain(t.clone()).walk(set, s),
            Self::Format3(t) => Ok(coverages(t.input_coverages(), set, s)?
                && coverages(t.backtrack_coverages(), set, s)?
                && coverages(t.lookahead_coverages(), set, s)?),
        }
    }
}

enum Context1<'a> {
    Plain(SequenceContextFormat1<'a>),
    Chain(ChainedSequenceContextFormat1<'a>),
}

enum RuleSet1<'a> {
    Plain(SequenceRuleSet<'a>),
    Chain(ChainedSequenceRuleSet<'a>),
}

enum Rule1<'a> {
    Plain(SequenceRule<'a>),
    Chain(ChainedSequenceRule<'a>),
}

impl Context1<'_> {
    fn coverage(&self) -> std::result::Result<CoverageTable<'_>, ReadError> {
        match self {
            Context1::Plain(table) => table.coverage(),
            Context1::Chain(table) => table.coverage(),
        }
    }

    fn rule_sets(
        &self,
    ) -> impl Iterator<Item = Option<std::result::Result<RuleSet1<'_>, ReadError>>> {
        let (left, right) = match self {
            Context1::Plain(table) => (
                Some(
                    table
                        .seq_rule_sets()
                        .iter()
                        .map(|rs| rs.map(|rs| rs.map(RuleSet1::Plain))),
                ),
                None,
            ),
            Context1::Chain(table) => (
                None,
                Some(
                    table
                        .chained_seq_rule_sets()
                        .iter()
                        .map(|rs| rs.map(|rs| rs.map(RuleSet1::Chain))),
                ),
            ),
        };
        left.into_iter()
            .flatten()
            .chain(right.into_iter().flatten())
    }
}

impl RuleSet1<'_> {
    fn rules(&self) -> impl Iterator<Item = std::result::Result<Rule1<'_>, ReadError>> {
        let (left, right) = match self {
            Self::Plain(table) => (
                Some(table.seq_rules().iter().map(|rule| rule.map(Rule1::Plain))),
                None,
            ),
            Self::Chain(table) => (
                None,
                Some(
                    table
                        .chained_seq_rules()
                        .iter()
                        .map(|rule| rule.map(Rule1::Chain)),
                ),
            ),
        };
        left.into_iter()
            .flatten()
            .chain(right.into_iter().flatten())
    }
}

enum Context2<'a> {
    Plain(SequenceContextFormat2<'a>),
    Chain(ChainedSequenceContextFormat2<'a>),
}

enum RuleSet2<'a> {
    Plain(ClassSequenceRuleSet<'a>),
    Chain(ChainedClassSequenceRuleSet<'a>),
}

enum Rule2<'a> {
    Plain(ClassSequenceRule<'a>),
    Chain(ChainedClassSequenceRule<'a>),
}

impl Context2<'_> {
    fn coverage(&self) -> std::result::Result<CoverageTable<'_>, ReadError> {
        match self {
            Context2::Plain(table) => table.coverage(),
            Context2::Chain(table) => table.coverage(),
        }
    }

    fn input_class_def(&self) -> std::result::Result<ClassDef<'_>, ReadError> {
        match self {
            Context2::Plain(table_ref) => table_ref.class_def(),
            Context2::Chain(table_ref) => table_ref.input_class_def(),
        }
    }

    fn rule_sets(
        &self,
    ) -> impl Iterator<Item = Option<std::result::Result<RuleSet2<'_>, ReadError>>> {
        let (left, right) = match self {
            Context2::Plain(table) => (
                Some(
                    table
                        .class_seq_rule_sets()
                        .iter()
                        .map(|rs| rs.map(|rs| rs.map(RuleSet2::Plain))),
                ),
                None,
            ),
            Context2::Chain(table) => (
                None,
                Some(
                    table
                        .chained_class_seq_rule_sets()
                        .iter()
                        .map(|rs| rs.map(|rs| rs.map(RuleSet2::Chain))),
                ),
            ),
        };
        left.into_iter()
            .flatten()
            .chain(right.into_iter().flatten())
    }
}

impl RuleSet2<'_> {
    fn rules(&self) -> impl Iterator<Item = std::result::Result<Rule2<'_>, ReadError>> {
        let (left, right) = match self {
            RuleSet2::Plain(table) => (
                Some(
                    table
                        .class_seq_rules()
                        .iter()
                        .map(|rule| rule.map(Rule2::Plain)),
                ),
                None,
            ),
            RuleSet2::Chain(table) => (
                None,
                Some(
                    table
                        .chained_class_seq_rules()
                        .iter()
                        .map(|rule| rule.map(Rule2::Chain)),
                ),
            ),
        };
        left.into_iter()
            .flatten()
            .chain(right.into_iter().flatten())
    }
}
