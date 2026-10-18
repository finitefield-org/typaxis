//! Shared allocation-free BASE, GDEF and kern grammar.
//! Each callback prepays a conservative fixed allowance before a local step.
use super::*;
pub(super) const STEP_WORK: u64 = 64;
#[derive(Debug)]
pub(super) enum Failure<E> {
    Native(Cff1Error),
    Budget(CffTableAdmissionBudgetFailureV2<E>),
}
impl<E> From<Cff1Error> for Failure<E> {
    fn from(error: Cff1Error) -> Self {
        Self::Native(error)
    }
}
pub(super) type Result<T, E> = std::result::Result<T, Failure<E>>;
pub(super) fn finish<T, E>(
    value: Result<T, E>,
) -> std::result::Result<std::result::Result<T, Cff1Error>, CffTableAdmissionBudgetFailureV2<E>> {
    match value {
        Ok(v) => Ok(Ok(v)),
        Err(Failure::Native(e)) => Ok(Err(e)),
        Err(Failure::Budget(e)) => Err(e),
    }
}
struct State<'a, B> {
    budget: &'a mut B,
    tag: Option<[u8; 4]>,
    length: usize,
    file_base: Option<usize>,
}
impl<B: CffAdmissionBudgetV2> State<'_, B> {
    fn work(
        &mut self,
        stage: CffTableAdmissionStageV2,
        offset: Option<usize>,
    ) -> Result<(), B::Error> {
        if !B::OBSERVE_OPTIONAL_LOCAL {
            return Ok(());
        }
        self.budget.work_optional_local(STEP_WORK).map_err(|error| {
            let table_offset = offset.filter(|n| *n <= self.length).map(|n| n as u64);
            Failure::Budget(CffTableAdmissionBudgetFailureV2 {
                error,
                point: CffTableAdmissionWorkPointV2 {
                    stage,
                    table_tag: self.tag,
                    table_offset,
                    file_offset: self
                        .file_base
                        .and_then(|base| table_offset.and_then(|n| (base as u64).checked_add(n))),
                },
            })
        })
    }
    fn u16(&mut self, bytes: &[u8], at: usize, error: Cff1Error) -> Result<u16, B::Error> {
        self.work(CffTableAdmissionStageV2::OptionalLocalField, Some(at))?;
        super::read_u16(bytes, at, error).map_err(Failure::Native)
    }
    fn u32(&mut self, bytes: &[u8], at: usize, error: Cff1Error) -> Result<u32, B::Error> {
        self.work(CffTableAdmissionStageV2::OptionalLocalField, Some(at))?;
        super::read_u32(bytes, at, error).map_err(Failure::Native)
    }
    fn range<'b>(
        &mut self,
        bytes: &'b [u8],
        at: usize,
        length: usize,
    ) -> Result<&'b [u8], B::Error> {
        self.work(CffTableAdmissionStageV2::OptionalLocalRange, Some(at))?;
        super::optional_range(bytes, at, length).map_err(Failure::Native)
    }
    fn relative(
        &mut self,
        bytes: &[u8],
        base: usize,
        raw: u16,
        nullable: bool,
    ) -> Result<Option<usize>, B::Error> {
        self.work(CffTableAdmissionStageV2::OptionalLocalOffset, Some(base))?;
        super::optional_relative(bytes, base, raw, nullable).map_err(Failure::Native)
    }
    fn relative32(
        &mut self,
        bytes: &[u8],
        base: usize,
        raw: u32,
        nullable: bool,
    ) -> Result<Option<usize>, B::Error> {
        self.work(CffTableAdmissionStageV2::OptionalLocalOffset, Some(base))?;
        super::optional_relative32(bytes, base, raw, nullable).map_err(Failure::Native)
    }
}
pub(super) fn validate<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyphs: u16,
    tag: [u8; 4],
    file_base: Option<usize>,
    budget: &mut B,
) -> Result<(), B::Error> {
    let mut state = State {
        budget,
        tag: Some(tag),
        length: bytes.len(),
        file_base,
    };
    match &tag {
        b"BASE" => validate_base_table_kernel(bytes, glyphs, &mut state),
        b"GDEF" => validate_gdef_table_kernel(bytes, glyphs, &mut state),
        b"kern" => validate_kern_table_kernel(bytes, glyphs, &mut state),
        _ => unreachable!("local validator received a different admitted tag"),
    }
}

fn validate_tag_records_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    start: usize,
    count: usize,
    record_size: usize,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    state.range(
        bytes,
        start,
        count
            .checked_mul(record_size)
            .ok_or(Cff1Error::InvalidOptionalTable)?,
    )?;
    let mut previous = None;
    for index in 0..count {
        let record = start + index * record_size;
        let tag: [u8; 4] = state
            .range(bytes, record, 4)?
            .try_into()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        if previous.is_some_and(|value| value >= tag) {
            return Err(Cff1Error::InvalidOptionalTable.into());
        }
        previous = Some(tag);
    }
    Ok(())
}

fn validate_coverage_table_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<usize, B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    match state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 => {
            let count =
                usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
            state.range(bytes, offset + 4, count * 2)?;
            let mut previous = None;
            for index in 0..count {
                let glyph = state.u16(
                    bytes,
                    offset + 4 + index * 2,
                    Cff1Error::InvalidOptionalTable,
                )?;
                if glyph >= glyph_count || previous.is_some_and(|value| value >= glyph) {
                    return Err(Cff1Error::InvalidOptionalTable.into());
                }
                previous = Some(glyph);
            }
            Ok(count)
        }
        2 => {
            let range_count =
                usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
            state.range(bytes, offset + 4, range_count * 6)?;
            let mut previous_end = None;
            let mut covered = 0usize;
            for index in 0..range_count {
                let record = offset + 4 + index * 6;
                let first = state.u16(bytes, record, Cff1Error::InvalidOptionalTable)?;
                let last = state.u16(bytes, record + 2, Cff1Error::InvalidOptionalTable)?;
                let start_index =
                    usize::from(state.u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?);
                if first > last
                    || last >= glyph_count
                    || previous_end.is_some_and(|value| value >= first)
                    || start_index != covered
                {
                    return Err(Cff1Error::InvalidOptionalTable.into());
                }
                covered = covered
                    .checked_add(usize::from(last - first) + 1)
                    .ok_or(Cff1Error::InvalidOptionalTable)?;
                previous_end = Some(last);
            }
            Ok(covered)
        }
        _ => Err(Cff1Error::InvalidOptionalTable.into()),
    }
}

fn validate_class_def_table_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    max_class: Option<u16>,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    match state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 => {
            let first = state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
            let count =
                usize::from(state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?);
            if usize::from(first)
                .checked_add(count)
                .map_or(true, |end| end > usize::from(glyph_count))
            {
                return Err(Cff1Error::InvalidOptionalTable.into());
            }
            state.range(bytes, offset + 6, count * 2)?;
            for index in 0..count {
                let class = state.u16(
                    bytes,
                    offset + 6 + index * 2,
                    Cff1Error::InvalidOptionalTable,
                )?;
                if max_class.is_some_and(|limit| class > limit) {
                    return Err(Cff1Error::InvalidOptionalTable.into());
                }
            }
        }
        2 => {
            let count =
                usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
            state.range(bytes, offset + 4, count * 6)?;
            let mut previous_end = None;
            for index in 0..count {
                let record = offset + 4 + index * 6;
                let first = state.u16(bytes, record, Cff1Error::InvalidOptionalTable)?;
                let last = state.u16(bytes, record + 2, Cff1Error::InvalidOptionalTable)?;
                let class = state.u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
                if first > last
                    || last >= glyph_count
                    || previous_end.is_some_and(|value| value >= first)
                    || max_class.is_some_and(|limit| class > limit)
                {
                    return Err(Cff1Error::InvalidOptionalTable.into());
                }
                previous_end = Some(last);
            }
        }
        _ => return Err(Cff1Error::InvalidOptionalTable.into()),
    }
    Ok(())
}

fn validate_device_table_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    let first = state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    let last = state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    let format = state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?;
    let bits = match format {
        1 => 2usize,
        2 => 4,
        3 => 8,
        _ => return Err(Cff1Error::InvalidOptionalTable.into()),
    };
    if first > last {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    let values = usize::from(last - first) + 1;
    let words = values
        .checked_mul(bits)
        .and_then(|value| value.checked_add(15))
        .map(|value| value / 16)
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    state.range(bytes, offset + 6, words * 2)?;
    Ok(())
}

fn validate_base_coord_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    match state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 => {
            state.range(bytes, offset, 4)?;
        }
        2 => {
            state.range(bytes, offset, 8)?;
            if state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)? >= glyph_count {
                return Err(Cff1Error::InvalidOptionalTable.into());
            }
        }
        3 => {
            state.range(bytes, offset, 6)?;
            let device = state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?;
            if let Some(device) = state.relative(bytes, offset, device, true)? {
                validate_device_table_kernel(bytes, device, state)?;
            }
        }
        _ => return Err(Cff1Error::InvalidOptionalTable.into()),
    }
    Ok(())
}

fn validate_base_min_max_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    state.range(bytes, offset, 6)?;
    for field in [0usize, 2] {
        let raw = state.u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(coord) = state.relative(bytes, offset, raw, true)? {
            validate_base_coord_kernel(bytes, coord, glyph_count, state)?;
        }
    }
    let count = usize::from(state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?);
    validate_tag_records_kernel(bytes, offset + 6, count, 8, state)?;
    for index in 0..count {
        let record = offset + 6 + index * 8;
        for field in [4usize, 6] {
            let raw = state.u16(bytes, record + field, Cff1Error::InvalidOptionalTable)?;
            if let Some(coord) = state.relative(bytes, offset, raw, true)? {
                validate_base_coord_kernel(bytes, coord, glyph_count, state)?;
            }
        }
    }
    Ok(())
}

fn validate_base_values_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    let default_index = usize::from(state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    let count = usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
    if count == 0 || default_index >= count {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    state.range(bytes, offset + 4, count * 2)?;
    for index in 0..count {
        let raw = state.u16(
            bytes,
            offset + 4 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let coord = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_base_coord_kernel(bytes, coord, glyph_count, state)?;
    }
    Ok(())
}

fn validate_base_script_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    state.range(bytes, offset, 6)?;
    let values = state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    if let Some(values) = state.relative(bytes, offset, values, true)? {
        validate_base_values_kernel(bytes, values, glyph_count, state)?;
    }
    let min_max = state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    if let Some(min_max) = state.relative(bytes, offset, min_max, true)? {
        validate_base_min_max_kernel(bytes, min_max, glyph_count, state)?;
    }
    let count = usize::from(state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?);
    validate_tag_records_kernel(bytes, offset + 6, count, 6, state)?;
    for index in 0..count {
        let record = offset + 6 + index * 6;
        let raw = state.u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let lang = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_base_min_max_kernel(bytes, lang, glyph_count, state)?;
    }
    Ok(())
}

fn validate_base_axis_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    state.range(bytes, offset, 4)?;
    let tags = state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    if let Some(tags) = state.relative(bytes, offset, tags, true)? {
        let count = usize::from(state.u16(bytes, tags, Cff1Error::InvalidOptionalTable)?);
        validate_tag_records_kernel(bytes, tags + 2, count, 4, state)?;
    }
    let scripts = state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    let scripts = state
        .relative(bytes, offset, scripts, false)?
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    let count = usize::from(state.u16(bytes, scripts, Cff1Error::InvalidOptionalTable)?);
    validate_tag_records_kernel(bytes, scripts + 2, count, 6, state)?;
    for index in 0..count {
        let record = scripts + 2 + index * 6;
        let raw = state.u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let script = state
            .relative(bytes, scripts, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_base_script_kernel(bytes, script, glyph_count, state)?;
    }
    Ok(())
}

fn validate_base_table_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    if bytes.len() < 8 || state.u32(bytes, 0, Cff1Error::InvalidOptionalTable)? != 0x0001_0000 {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    for field in [4usize, 6] {
        let raw = state.u16(bytes, field, Cff1Error::InvalidOptionalTable)?;
        if let Some(axis) = state.relative(bytes, 0, raw, true)? {
            validate_base_axis_kernel(bytes, axis, glyph_count, state)?;
        }
    }
    Ok(())
}

fn validate_attach_list_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    let coverage_raw = state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    let count = usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
    let coverage = state
        .relative(bytes, offset, coverage_raw, false)?
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    if validate_coverage_table_kernel(bytes, coverage, glyph_count, state)? != count {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    state.range(bytes, offset + 4, count * 2)?;
    for index in 0..count {
        let raw = state.u16(
            bytes,
            offset + 4 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let point = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        let point_count = usize::from(state.u16(bytes, point, Cff1Error::InvalidOptionalTable)?);
        state.range(bytes, point + 2, point_count * 2)?;
    }
    Ok(())
}

fn validate_caret_value_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    match state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 | 2 => {
            state.range(bytes, offset, 4)?;
        }
        3 => {
            state.range(bytes, offset, 6)?;
            let raw = state.u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?;
            let device = state
                .relative(bytes, offset, raw, false)?
                .ok_or(Cff1Error::InvalidOptionalTable)?;
            validate_device_table_kernel(bytes, device, state)?;
        }
        _ => return Err(Cff1Error::InvalidOptionalTable.into()),
    }
    Ok(())
}

fn validate_lig_caret_list_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    let coverage_raw = state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    let count = usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
    let coverage = state
        .relative(bytes, offset, coverage_raw, false)?
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    if validate_coverage_table_kernel(bytes, coverage, glyph_count, state)? != count {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    state.range(bytes, offset + 4, count * 2)?;
    for index in 0..count {
        let raw = state.u16(
            bytes,
            offset + 4 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let ligature = state
            .relative(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        let caret_count =
            usize::from(state.u16(bytes, ligature, Cff1Error::InvalidOptionalTable)?);
        if caret_count == 0 {
            return Err(Cff1Error::InvalidOptionalTable.into());
        }
        state.range(bytes, ligature + 2, caret_count * 2)?;
        for caret in 0..caret_count {
            let raw = state.u16(
                bytes,
                ligature + 2 + caret * 2,
                Cff1Error::InvalidOptionalTable,
            )?;
            let value = state
                .relative(bytes, ligature, raw, false)?
                .ok_or(Cff1Error::InvalidOptionalTable)?;
            validate_caret_value_kernel(bytes, value, state)?;
        }
    }
    Ok(())
}

fn validate_mark_glyph_sets_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    if state.u16(bytes, offset, Cff1Error::InvalidOptionalTable)? != 1 {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    let count = usize::from(state.u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?);
    state.range(bytes, offset + 4, count * 4)?;
    for index in 0..count {
        let raw = state.u32(
            bytes,
            offset + 4 + index * 4,
            Cff1Error::InvalidOptionalTable,
        )?;
        let coverage = state
            .relative32(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_coverage_table_kernel(bytes, coverage, glyph_count, state)?;
    }
    Ok(())
}

fn validate_gdef_table_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    let version = state.u32(bytes, 0, Cff1Error::InvalidOptionalTable)?;
    let header_length = match version {
        0x0001_0000 => 12,
        0x0001_0002 => 14,
        _ => return Err(Cff1Error::InvalidOptionalTable.into()),
    };
    state.range(bytes, 0, header_length)?;
    let glyph_class = state.u16(bytes, 4, Cff1Error::InvalidOptionalTable)?;
    if let Some(class) = state.relative(bytes, 0, glyph_class, true)? {
        validate_class_def_table_kernel(bytes, class, glyph_count, Some(4), state)?;
    }
    let attach = state.u16(bytes, 6, Cff1Error::InvalidOptionalTable)?;
    if let Some(attach) = state.relative(bytes, 0, attach, true)? {
        validate_attach_list_kernel(bytes, attach, glyph_count, state)?;
    }
    let carets = state.u16(bytes, 8, Cff1Error::InvalidOptionalTable)?;
    if let Some(carets) = state.relative(bytes, 0, carets, true)? {
        validate_lig_caret_list_kernel(bytes, carets, glyph_count, state)?;
    }
    let mark_class = state.u16(bytes, 10, Cff1Error::InvalidOptionalTable)?;
    if let Some(class) = state.relative(bytes, 0, mark_class, true)? {
        validate_class_def_table_kernel(bytes, class, glyph_count, None, state)?;
    }
    if version == 0x0001_0002 {
        let sets = state.u16(bytes, 12, Cff1Error::InvalidOptionalTable)?;
        if let Some(sets) = state.relative(bytes, 0, sets, true)? {
            validate_mark_glyph_sets_kernel(bytes, sets, glyph_count, state)?;
        }
    }
    Ok(())
}

fn validate_kern_table_kernel<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(CffTableAdmissionStageV2::OptionalLocalHeader, None)?;
    if state.u16(bytes, 0, Cff1Error::InvalidOptionalTable)? != 0 {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    let count = usize::from(state.u16(bytes, 2, Cff1Error::InvalidOptionalTable)?);
    let mut cursor = 4usize;
    for _ in 0..count {
        state.range(bytes, cursor, 14)?;
        let version = state.u16(bytes, cursor, Cff1Error::InvalidOptionalTable)?;
        let length = usize::from(state.u16(bytes, cursor + 2, Cff1Error::InvalidOptionalTable)?);
        let coverage = state.u16(bytes, cursor + 4, Cff1Error::InvalidOptionalTable)?;
        let pair_count =
            usize::from(state.u16(bytes, cursor + 6, Cff1Error::InvalidOptionalTable)?);
        let expected_length = 14usize
            .checked_add(
                pair_count
                    .checked_mul(6)
                    .ok_or(Cff1Error::InvalidOptionalTable)?,
            )
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        let power = if pair_count == 0 {
            0
        } else {
            1usize << (usize::BITS - 1 - pair_count.leading_zeros())
        };
        let expected_search = power * 6;
        let expected_selector = if power == 0 {
            0
        } else {
            usize::try_from(power.trailing_zeros()).map_err(|_| Cff1Error::InvalidOptionalTable)?
        };
        let expected_shift = pair_count * 6 - expected_search;
        if version != 0
            || coverage >> 8 != 0
            || coverage & !0x000F != 0
            || coverage & 0x0001 == 0
            || length != expected_length
            || usize::from(state.u16(bytes, cursor + 8, Cff1Error::InvalidOptionalTable)?)
                != expected_search
            || usize::from(state.u16(bytes, cursor + 10, Cff1Error::InvalidOptionalTable)?)
                != expected_selector
            || usize::from(state.u16(bytes, cursor + 12, Cff1Error::InvalidOptionalTable)?)
                != expected_shift
        {
            return Err(Cff1Error::InvalidOptionalTable.into());
        }
        state.range(bytes, cursor, length)?;
        let mut previous = None;
        for index in 0..pair_count {
            let pair = cursor + 14 + index * 6;
            let left = state.u16(bytes, pair, Cff1Error::InvalidOptionalTable)?;
            let right = state.u16(bytes, pair + 2, Cff1Error::InvalidOptionalTable)?;
            if left >= glyph_count
                || right >= glyph_count
                || previous.is_some_and(|value| value >= (left, right))
            {
                return Err(Cff1Error::InvalidOptionalTable.into());
            }
            previous = Some((left, right));
        }
        cursor = cursor
            .checked_add(length)
            .ok_or(Cff1Error::InvalidOptionalTable)?;
    }
    if cursor != bytes.len() {
        return Err(Cff1Error::InvalidOptionalTable.into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "cff_local_optional_tests.rs"]
mod tests;
