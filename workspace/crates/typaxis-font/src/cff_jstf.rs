//! Shared JSTF grammar. Observed GPOS intersections use borrowed readers and
//! fixed stack sets; ordinary admission retains the locked library algorithm.
use super::*;
use local_optional::{Failure, Result};
use CffTableAdmissionStageV2 as Stage;
pub(super) const STEP_WORK: u64 = 64;
/// Constant-time borrowed readers inspect at most a fixed header and array views.
/// Binary searches have at most 16 comparisons (all counts are u16).
const READER_WORK: u64 = 512;
struct State<'a, B> {
    budget: &'a mut B,
    start: usize,
    length: usize,
    base: Option<usize>,
}
impl<B: CffAdmissionBudgetV2> State<'_, B> {
    fn charge(
        &mut self,
        stage: Stage,
        at: Option<usize>,
        amount: u64,
    ) -> std::result::Result<(), CffTableAdmissionBudgetFailureV2<B::Error>> {
        if !B::OBSERVE_JSTF {
            return Ok(());
        }
        self.budget.work_jstf(amount).map_err(|error| {
            let table_offset = at.filter(|n| *n <= self.length).map(|n| n as u64);
            CffTableAdmissionBudgetFailureV2 {
                error,
                point: CffTableAdmissionWorkPointV2 {
                    stage,
                    table_tag: Some(*b"JSTF"),
                    table_offset,
                    file_offset: self
                        .base
                        .and_then(|b| table_offset.and_then(|n| (b as u64).checked_add(n))),
                },
            }
        })
    }
    fn step(&mut self, stage: Stage, at: Option<usize>) -> Result<(), B::Error> {
        self.charge(stage, at, STEP_WORK).map_err(Failure::Budget)
    }
    fn u16(&mut self, bytes: &[u8], at: usize, error: Cff1Error) -> Result<u16, B::Error> {
        self.step(Stage::JstfField, Some(at))?;
        super::read_u16(bytes, at, error).map_err(Failure::Native)
    }
    fn u32(&mut self, bytes: &[u8], at: usize, error: Cff1Error) -> Result<u32, B::Error> {
        self.step(Stage::JstfField, Some(at))?;
        super::read_u32(bytes, at, error).map_err(Failure::Native)
    }
    fn range<'b>(
        &mut self,
        bytes: &'b [u8],
        at: usize,
        length: usize,
    ) -> Result<&'b [u8], B::Error> {
        self.step(Stage::JstfRange, Some(at))?;
        super::optional_range(bytes, at, length).map_err(Failure::Native)
    }
    fn relative(
        &mut self,
        bytes: &[u8],
        base: usize,
        raw: u16,
        nullable: bool,
    ) -> Result<Option<usize>, B::Error> {
        self.step(Stage::JstfOffset, Some(base))?;
        super::optional_relative(bytes, base, raw, nullable).map_err(Failure::Native)
    }
}
pub(super) fn validate<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyphs: u16,
    gsub: u16,
    gpos: u16,
    base: Option<usize>,
    budget: &mut B,
) -> Result<(), B::Error> {
    let mut state = State {
        budget,
        start: bytes.as_ptr() as usize,
        length: bytes.len(),
        base,
    };
    validate_jstf_table(bytes, glyphs, gsub, gpos, &mut state)
}

#[path = "cff_jstf_intersection.rs"]
mod intersection;

fn validate_jstf_mod_list<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    lookup_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    let count = usize::from(state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    state.range(bytes, offset + 2, count * 2)?;
    let mut previous = None;
    for index in 0..count {
        let lookup = state.u16(
            bytes,
            offset + 2 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        if lookup >= lookup_count || previous.is_some_and(|value| value >= lookup) {
            return Err(Cff1Error::InvalidOptionalTable.into());
        }
        previous = Some(lookup);
    }
    Ok(())
}

fn validate_jstf_max<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    let count = usize::from(state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    state.range(bytes, offset + 2, count * 2)?;
    // The ordinary API retains its library set and intersection implementation.
    let glyphs = if B::OBSERVE_JSTF {
        None
    } else {
        let mut set = IntSet::empty();
        for gid in 0..glyph_count {
            set.insert(GlyphId::new(u32::from(gid)));
        }
        Some(set)
    };
    for index in 0..count {
        let raw = state.u16(
            bytes,
            offset + 2 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let lookup = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        if let Some(glyphs) = &glyphs {
            PositionLookup::read(FontData::new(&bytes[lookup..]))
                .and_then(|l| l.subtables())
                .and_then(|s| s.intersects(glyphs))
                .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        } else {
            intersection::lookup(&bytes[lookup..], glyph_count, state).map_err(|e| match e {
                intersection::Failure::Native(error) => {
                    let _ = error;
                    Failure::Native(Cff1Error::InvalidOptionalTable)
                }
                intersection::Failure::Budget(e) => Failure::Budget(e),
            })?;
        }
    }
    Ok(())
}

fn validate_jstf_priority<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    state.range(bytes, offset, 20)?;
    for field in [0usize, 2, 10, 12] {
        let raw = state.u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(list) = state.relative(bytes, offset, raw, true)? {
            validate_jstf_mod_list(bytes, list, gsub_lookup_count, state)?;
        }
    }
    for field in [4usize, 6, 14, 16] {
        let raw = state.u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(list) = state.relative(bytes, offset, raw, true)? {
            validate_jstf_mod_list(bytes, list, gpos_lookup_count, state)?;
        }
    }
    for field in [8usize, 18] {
        let raw = state.u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(max) = state.relative(bytes, offset, raw, true)? {
            validate_jstf_max(bytes, max, glyph_count, state)?;
        }
    }
    Ok(())
}

fn validate_jstf_lang_sys<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    let count = usize::from(state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    state.range(bytes, offset + 2, count * 2)?;
    for index in 0..count {
        let raw = state.u16(
            bytes,
            offset + 2 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let priority = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_jstf_priority(
            bytes,
            priority,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
            state,
        )?;
    }
    Ok(())
}

fn validate_jstf_script<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    state.range(bytes, offset, 6)?;
    let extenders = state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    if let Some(extenders) = state.relative(bytes, offset, extenders, true)? {
        let count = usize::from(state.u16(bytes, extenders, Cff1Error::InvalidOptionalTable)?);
        state.range(bytes, extenders + 2, count * 2)?;
        let mut previous = None;
        for index in 0..count {
            let glyph = state.u16(
                bytes,
                extenders + 2 + index * 2,
                Cff1Error::InvalidOptionalTable,
            )?;
            if glyph >= glyph_count || previous.is_some_and(|value| value >= glyph) {
                return Err(Cff1Error::InvalidOptionalTable.into());
            }
            previous = Some(glyph);
        }
    }
    let default = state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    if let Some(default) = state.relative(bytes, offset, default, true)? {
        validate_jstf_lang_sys(
            bytes,
            default,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
            state,
        )?;
    }
    let count = usize::from(state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?);
    validate_tags(bytes, offset + 6, count, state)?;
    for index in 0..count {
        let record = offset + 6 + index * 6;
        let raw = state.u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let lang = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_jstf_lang_sys(
            bytes,
            lang,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
            state,
        )?;
    }
    Ok(())
}

fn validate_jstf_table<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    if bytes.len() < 6 || state.u32(bytes, 0, Cff1Error::InvalidOptionalTable)? != 0x0001_0000 {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    let count = usize::from(state.u16(bytes, 4, Cff1Error::InvalidOptionalTable)?);
    validate_tags(bytes, 6, count, state)?;
    for index in 0..count {
        let record = 6 + index * 6;
        let raw = state.u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let script = state
            .relative(bytes, 0, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_jstf_script(
            bytes,
            script,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
            state,
        )?;
    }
    Ok(())
}

fn validate_tags<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    start: usize,
    count: usize,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.step(Stage::JstfHeader, None)?;
    state.range(
        bytes,
        start,
        count
            .checked_mul(6)
            .ok_or(Cff1Error::InvalidOptionalTable)?,
    )?;
    let mut previous = None;
    for i in 0..count {
        let tag: [u8; 4] = state
            .range(bytes, start + i * 6, 4)?
            .try_into()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        if previous.is_some_and(|p| p >= tag) {
            return Err(Cff1Error::InvalidOptionalTable.into());
        }
        previous = Some(tag);
    }
    Ok(())
}

#[cfg(test)]
#[path = "cff_jstf_tests.rs"]
mod tests;
