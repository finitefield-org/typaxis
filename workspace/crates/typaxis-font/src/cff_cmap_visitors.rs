//! The existing CFF base-cmap validation with allocation delegated to the caller.
use super::*;

pub(super) fn visit_format4<E: From<Cff1Error>>(
    table: &[u8],
    offset: usize,
    glyph_count: u16,
    mut emit: impl FnMut(u32, u16) -> Result<(), E>,
    work: &mut impl FnMut(u64) -> Result<(), E>,
) -> Result<(), E> {
    work(1)?;
    let length = usize::from(read_u16(table, offset + 2, Cff1Error::InvalidCmap)?);
    let end = offset.checked_add(length).ok_or(Cff1Error::InvalidCmap)?;
    if length < 24 || end > table.len() || read_u16(table, offset + 4, Cff1Error::InvalidCmap)? != 0
    {
        return Err(Cff1Error::InvalidCmap.into());
    }
    let seg_count_x2 = read_u16(table, offset + 6, Cff1Error::InvalidCmap)?;
    if seg_count_x2 == 0 || seg_count_x2 % 2 != 0 {
        return Err(Cff1Error::InvalidCmap.into());
    }
    let seg_count = usize::from(seg_count_x2 / 2);
    let power = 1usize << (usize::BITS - 1 - seg_count.leading_zeros());
    if read_u16(table, offset + 8, Cff1Error::InvalidCmap)?
        != u16::try_from(power * 2).map_err(|_| Cff1Error::InvalidCmap)?
        || read_u16(table, offset + 10, Cff1Error::InvalidCmap)?
            != u16::try_from(power.trailing_zeros()).map_err(|_| Cff1Error::InvalidCmap)?
        || read_u16(table, offset + 12, Cff1Error::InvalidCmap)?
            != seg_count_x2 - u16::try_from(power * 2).map_err(|_| Cff1Error::InvalidCmap)?
    {
        return Err(Cff1Error::InvalidCmap.into());
    }
    let end_codes = offset + 14;
    let reserved_pad = end_codes + seg_count * 2;
    let start_codes = reserved_pad + 2;
    let deltas = start_codes + seg_count * 2;
    let range_offsets = deltas + seg_count * 2;
    let arrays_end = range_offsets + seg_count * 2;
    if arrays_end > end || read_u16(table, reserved_pad, Cff1Error::InvalidCmap)? != 0 {
        return Err(Cff1Error::InvalidCmap.into());
    }
    let mut previous_end = None;
    for segment in 0..seg_count {
        work(1)?;
        let start = read_u16(table, start_codes + segment * 2, Cff1Error::InvalidCmap)?;
        let finish = read_u16(table, end_codes + segment * 2, Cff1Error::InvalidCmap)?;
        if start > finish || previous_end.is_some_and(|previous| previous >= start) {
            return Err(Cff1Error::InvalidCmap.into());
        }
        previous_end = Some(finish);
        let delta = read_i16(table, deltas + segment * 2, Cff1Error::InvalidCmap)?;
        let range = read_u16(table, range_offsets + segment * 2, Cff1Error::InvalidCmap)?;
        for scalar in start..=finish {
            work(1)?;
            let gid = if range == 0 {
                scalar.wrapping_add_signed(delta)
            } else {
                let word_position = range_offsets
                    .checked_add(segment * 2)
                    .and_then(|value| value.checked_add(usize::from(range)))
                    .and_then(|value| {
                        value.checked_add(usize::from(scalar - start).checked_mul(2)?)
                    })
                    .ok_or(Cff1Error::InvalidCmap)?;
                if word_position + 2 > end {
                    return Err(Cff1Error::InvalidCmap.into());
                }
                let indexed = read_u16(table, word_position, Cff1Error::InvalidCmap)?;
                if indexed == 0 {
                    0
                } else {
                    indexed.wrapping_add_signed(delta)
                }
            };
            if gid != 0 {
                let scalar32 = u32::from(scalar);
                if (0xD800..=0xDFFF).contains(&scalar32) || gid >= glyph_count {
                    return Err(Cff1Error::InvalidCmap.into());
                }
                emit(scalar32, gid)?;
            }
        }
    }
    if previous_end != Some(0xFFFF) {
        return Err(Cff1Error::InvalidCmap.into());
    }
    Ok(())
}

pub(super) fn visit_format12<E: From<Cff1Error>>(
    table: &[u8],
    offset: usize,
    glyph_count: u16,
    mut emit: impl FnMut(u32, u16) -> Result<(), E>,
    work: &mut impl FnMut(u64) -> Result<(), E>,
) -> Result<(), E> {
    work(1)?;
    if read_u16(table, offset + 2, Cff1Error::InvalidCmap)? != 0 {
        return Err(Cff1Error::InvalidCmap.into());
    }
    let length = usize::try_from(read_u32(table, offset + 4, Cff1Error::InvalidCmap)?)
        .map_err(|_| Cff1Error::InvalidCmap)?;
    let end = offset.checked_add(length).ok_or(Cff1Error::InvalidCmap)?;
    let groups = usize::try_from(read_u32(table, offset + 12, Cff1Error::InvalidCmap)?)
        .map_err(|_| Cff1Error::InvalidCmap)?;
    if length < 16
        || end > table.len()
        || 16usize.checked_add(groups.checked_mul(12).ok_or(Cff1Error::InvalidCmap)?)
            != Some(length)
    {
        return Err(Cff1Error::InvalidCmap.into());
    }
    let mut previous_end = None;
    for index in 0..groups {
        work(1)?;
        let group = offset + 16 + index * 12;
        let start = read_u32(table, group, Cff1Error::InvalidCmap)?;
        let finish = read_u32(table, group + 4, Cff1Error::InvalidCmap)?;
        let start_gid = read_u32(table, group + 8, Cff1Error::InvalidCmap)?;
        if start > finish
            || finish > 0x10FFFF
            || previous_end.is_some_and(|previous| previous >= start)
            || (start <= 0xDFFF && finish >= 0xD800)
        {
            return Err(Cff1Error::InvalidCmap.into());
        }
        previous_end = Some(finish);
        for scalar in start..=finish {
            work(1)?;
            let gid = start_gid
                .checked_add(scalar - start)
                .ok_or(Cff1Error::InvalidCmap)?;
            if gid != 0 {
                if gid >= u32::from(glyph_count) {
                    return Err(Cff1Error::InvalidCmap.into());
                }
                emit(
                    scalar,
                    u16::try_from(gid).map_err(|_| Cff1Error::InvalidCmap)?,
                )?;
            }
        }
    }
    Ok(())
}
