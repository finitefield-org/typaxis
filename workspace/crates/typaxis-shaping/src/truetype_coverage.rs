//! Observed nominal coverage matches read-fonts 0.31.3's first-Some mapper.
//! Only scalar coverage is authorized here. No lookup allocation, output credit,
//! font selection, or PDF authority is added. Review this kernel if its pin moves.
use super::*;
use crate::shared_shape_storage::Storage;
use read_fonts::tables::cmap::{Cmap, Cmap4, CmapSubtable};
type E = StagingEquationNumberShapeError;

fn work<S: Storage>(storage: &mut S, amount: u64) -> Result<(), E> {
    storage.work(amount).map_err(E::Backend)
}

pub(super) fn validate<S: Storage>(
    admitted: &AdmittedFont,
    text: &str,
    storage: &mut S,
) -> Result<(), E> {
    let cmap = load_cmap(admitted.bytes(), admitted.face_index(), storage)?;
    let mut chars = text.chars();
    loop {
        work(storage, 1)?;
        let Some(character) = chars.next() else { break };
        if is_shaping_default_ignorable(character) {
            continue;
        }
        let glyph = map_codepoint(&cmap, character as u32, storage)?
            .ok_or(E::MissingDeclaredFontCoverage)?;
        if glyph == 0 || glyph >= admitted.metadata().glyph_count {
            return Err(E::MissingDeclaredFontCoverage);
        }
    }
    Ok(())
}

pub(super) fn load_cmap<'a, S: Storage>(
    bytes: &'a [u8],
    index: u32,
    storage: &mut S,
) -> Result<Cmap<'a>, E> {
    let face =
        crate::font_reader::read_with_tables(bytes, index, 1, storage).map_err(|e| match e {
            LinkedShaperError::InvalidFontOrFace => E::InvalidFontOrFace,
            e => E::Backend(e),
        })?;
    face.cmap().map_err(|_| E::InvalidFontOrFace)
}

pub(super) fn map_codepoint<S: Storage>(
    cmap: &Cmap<'_>,
    codepoint: u32,
    storage: &mut S,
) -> Result<Option<u32>, E> {
    let mut records = cmap.encoding_records().iter();
    loop {
        work(storage, 1)?;
        let Some(record) = records.next() else {
            return Ok(None);
        };
        let Ok(subtable) = record.subtable(cmap.offset_data()) else {
            continue;
        };
        let glyph = match subtable {
            CmapSubtable::Format0(v) => {
                work(storage, 1)?;
                v.map_codepoint(codepoint).map(|g| g.to_u32())
            }
            CmapSubtable::Format6(v) => {
                work(storage, 1)?;
                v.map_codepoint(codepoint).map(|g| g.to_u32())
            }
            CmapSubtable::Format4(v) => format4(&v, codepoint, storage)?,
            CmapSubtable::Format12(v) => groups(
                v.groups().len(),
                codepoint,
                false,
                |i| {
                    let g = v.groups().get(i)?;
                    Some((g.start_char_code(), g.end_char_code(), g.start_glyph_id()))
                },
                storage,
            )?,
            CmapSubtable::Format13(v) => groups(
                v.groups().len(),
                codepoint,
                true,
                |i| {
                    let g = v.groups().get(i)?;
                    Some((g.start_char_code(), g.end_char_code(), g.glyph_id()))
                },
                storage,
            )?,
            _ => None,
        };
        if glyph.is_some() {
            return Ok(glyph);
        }
    }
}

fn format4<S: Storage>(
    table: &Cmap4<'_>,
    codepoint: u32,
    storage: &mut S,
) -> Result<Option<u32>, E> {
    work(storage, 1)?;
    if codepoint > 0xffff {
        return Ok(None);
    }
    let cp = codepoint as u16;
    let mut lo = 0;
    let mut hi = usize::from(table.seg_count_x2()) / 2;
    while lo < hi {
        work(storage, 1)?;
        let i = lo + (hi - lo) / 2;
        let Some(start) = table.start_code().get(i).map(|v| v.get()) else {
            return Ok(None);
        };
        if cp < start {
            hi = i;
            continue;
        }
        let Some(end) = table.end_code().get(i).map(|v| v.get()) else {
            return Ok(None);
        };
        if cp > end {
            lo = i + 1;
            continue;
        }
        work(storage, 1)?;
        let Some(delta) = table.id_delta().get(i).map(|v| i32::from(v.get())) else {
            return Ok(None);
        };
        let offsets = table.id_range_offsets();
        let Some(offset) = offsets.get(i).map(|v| usize::from(v.get())) else {
            return Ok(None);
        };
        if offset == 0 {
            return Ok(Some(u32::from((i32::from(cp) + delta) as u16)));
        }
        let at = (offset / 2 + usize::from(cp - start)).saturating_sub(offsets.len() - i);
        let Some(gid) = table.glyph_id_array().get(at).map(|v| v.get()) else {
            return Ok(None);
        };
        return Ok((gid != 0).then_some(u32::from((i32::from(gid) + delta) as u16)));
    }
    Ok(None)
}

fn groups<S: Storage>(
    count: usize,
    codepoint: u32,
    constant: bool,
    read: impl Fn(usize) -> Option<(u32, u32, u32)>,
    storage: &mut S,
) -> Result<Option<u32>, E> {
    let mut lo = 0;
    let mut hi = count;
    while lo < hi {
        work(storage, 1)?;
        let i = lo + (hi - lo) / 2;
        let Some((start, end, gid)) = read(i) else {
            return Ok(None);
        };
        if codepoint < start {
            hi = i;
        } else if codepoint > end {
            lo = i + 1;
        } else {
            return Ok(Some(if constant {
                gid
            } else {
                gid.wrapping_add(codepoint.wrapping_sub(start))
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "truetype_coverage_tests.rs"]
mod tests;
