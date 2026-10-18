//! Shared allocation-free MATH table validation. Ordinary callers disable hooks.
use super::*;
pub(crate) const STEP_WORK: u64 = 64;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Stage {
    Header,
    Field,
    RelativeOffset,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Point {
    pub stage: Stage,
    pub table_offset: Option<u64>,
}
pub(crate) trait Budget {
    type Error;
    const OBSERVED: bool = true;
    fn work(&mut self, point: Point, amount: u64) -> std::result::Result<(), Self::Error>;
}
pub(super) struct Unobserved;
impl Budget for Unobserved {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn work(&mut self, _: Point, _: u64) -> std::result::Result<(), Self::Error> {
        unreachable!()
    }
}
#[derive(Debug)]
pub(super) enum Failure<E> {
    Native(MathFontError),
    Budget(E),
}
impl<E> From<MathFontError> for Failure<E> {
    fn from(e: MathFontError) -> Self {
        Self::Native(e)
    }
}
type Result<T, E> = std::result::Result<T, Failure<E>>;
pub(crate) type BudgetResult<T, E> = std::result::Result<std::result::Result<T, MathFontError>, E>;
fn finish<T, E>(value: Result<T, E>) -> BudgetResult<T, E> {
    match value {
        Ok(v) => Ok(Ok(v)),
        Err(Failure::Native(e)) => Ok(Err(e)),
        Err(Failure::Budget(e)) => Err(e),
    }
}
/// Legacy keeps the published strict ordering. Private CFF /2 also accepts
/// equal measurements for distinct ready-made variants, preserving their order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VariantOrder {
    LegacyStrict,
    CffV2Nondecreasing,
}
struct State<'a, B> {
    budget: &'a mut B,
    length: usize,
    variant_order: VariantOrder,
}
impl<B: Budget> State<'_, B> {
    fn work(&mut self, stage: Stage, offset: Option<usize>) -> Result<(), B::Error> {
        if !B::OBSERVED {
            return Ok(());
        }
        let point = Point {
            stage,
            table_offset: offset.filter(|n| *n <= self.length).map(|n| n as u64),
        };
        self.budget.work(point, STEP_WORK).map_err(Failure::Budget)
    }
    fn u16(&mut self, table: &[u8], offset: usize) -> Result<Option<u16>, B::Error> {
        self.work(Stage::Field, Some(offset))?;
        Ok(super::read_u16(table, offset))
    }
    fn i16(&mut self, table: &[u8], offset: usize) -> Result<Option<i16>, B::Error> {
        self.work(Stage::Field, Some(offset))?;
        Ok(super::read_i16(table, offset))
    }
    fn u32(&mut self, table: &[u8], offset: usize) -> Result<Option<u32>, B::Error> {
        self.work(Stage::Field, Some(offset))?;
        Ok(super::read_u32(table, offset))
    }
    fn relative(&mut self, base: usize, offset: usize, length: usize) -> Result<usize, B::Error> {
        self.work(Stage::RelativeOffset, Some(base))?;
        super::checked_relative(base, offset, length).map_err(Failure::Native)
    }
}
pub(super) fn table<B: Budget>(
    bytes: &[u8],
    glyphs: u16,
    budget: &mut B,
) -> BudgetResult<MathTableInfo, B::Error> {
    table_with_variant_order(bytes, glyphs, VariantOrder::LegacyStrict, budget)
}
pub(super) fn table_with_variant_order<B: Budget>(
    bytes: &[u8],
    glyphs: u16,
    variant_order: VariantOrder,
    budget: &mut B,
) -> BudgetResult<MathTableInfo, B::Error> {
    let mut state = State {
        budget,
        length: bytes.len(),
        variant_order,
    };
    finish(validate_math_table_kernel(bytes, glyphs, &mut state))
}

fn validate_math_table_kernel<B: Budget>(
    table: &[u8],
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<MathTableInfo, B::Error> {
    state.work(Stage::Header, None)?;
    if table.len() < 10 {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    if state.u32(table, 0)? != Some(0x0001_0000) {
        return Err(MathFontError::UnsupportedMathVersion.into());
    }
    let constants_offset = usize::from(
        state
            .u16(table, 4)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let glyph_info_offset = usize::from(
        state
            .u16(table, 6)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let variants_offset = usize::from(
        state
            .u16(table, 8)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if constants_offset < 10
        || constants_offset
            .checked_add(MATH_CONSTANTS_LENGTH)
            .map_or(true, |end| end > table.len())
        || glyph_info_offset < 10
        || glyph_info_offset
            .checked_add(8)
            .map_or(true, |end| end > table.len())
        || variants_offset < 10
        || variants_offset
            .checked_add(10)
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    let script = state
        .i16(table, constants_offset)?
        .ok_or(MathFontError::MalformedTable("MATH"))?;
    let script_script = state
        .i16(table, constants_offset + 2)?
        .ok_or(MathFontError::MalformedTable("MATH"))?;
    let display_operator_min_height = state
        .u16(table, constants_offset + 6)?
        .ok_or(MathFontError::MalformedTable("MATH"))?;
    if !(1..=100).contains(&script)
        || !(1..=100).contains(&script_script)
        || display_operator_min_height == 0
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    for index in 0..MATH_VALUE_RECORD_COUNT {
        let record = constants_offset + 8 + index * 4;
        if state.u16(table, record + 2)? != Some(0) {
            return Err(MathFontError::UnsupportedMathDeviceTable.into());
        }
    }
    validate_math_glyph_info_kernel(table, glyph_info_offset, glyph_count, state)?;
    validate_math_variants_kernel(table, variants_offset, glyph_count, state)?;
    let mut value = |index: usize| -> Result<i16, B::Error> {
        state
            .i16(table, constants_offset + 8 + index * 4)?
            .ok_or(MathFontError::MalformedTable("MATH"))
            .map_err(Failure::Native)
    };
    let constants = MathFontConstants {
        script_percent_scale_down: u16::try_from(script)
            .map_err(|_| MathFontError::MalformedTable("MATH"))?,
        script_script_percent_scale_down: u16::try_from(script_script)
            .map_err(|_| MathFontError::MalformedTable("MATH"))?,
        display_operator_min_height,
        axis_height: value(1)?,
        subscript_shift_down: value(4)?,
        subscript_top_max: value(5)?,
        subscript_baseline_drop_min: value(6)?,
        superscript_shift_up: value(7)?,
        superscript_shift_up_cramped: value(8)?,
        superscript_bottom_min: value(9)?,
        superscript_baseline_drop_max: value(10)?,
        sub_superscript_gap_min: value(11)?,
        superscript_bottom_max_with_subscript: value(12)?,
        space_after_script: value(13)?,
        fraction_numerator_shift_up: value(28)?,
        fraction_numerator_display_shift_up: value(29)?,
        fraction_denominator_shift_down: value(30)?,
        fraction_denominator_display_shift_down: value(31)?,
        fraction_numerator_gap_min: value(32)?,
        fraction_numerator_display_gap_min: value(33)?,
        fraction_rule_thickness: value(34)?,
        fraction_denominator_gap_min: value(35)?,
        fraction_denominator_display_gap_min: value(36)?,
        radical_vertical_gap: value(45)?,
        radical_display_vertical_gap: value(46)?,
        radical_rule_thickness: value(47)?,
    };
    if constants.subscript_shift_down < 0
        || constants.subscript_top_max < 0
        || constants.subscript_baseline_drop_min < 0
        || constants.superscript_shift_up < 0
        || constants.superscript_shift_up_cramped < 0
        || constants.superscript_bottom_min < 0
        || constants.superscript_baseline_drop_max < 0
        || constants.sub_superscript_gap_min < 0
        || constants.superscript_bottom_max_with_subscript < 0
        || constants.space_after_script < 0
        || constants.fraction_numerator_shift_up < 0
        || constants.fraction_numerator_display_shift_up < 0
        || constants.fraction_denominator_shift_down < 0
        || constants.fraction_denominator_display_shift_down < 0
        || constants.fraction_numerator_gap_min < 0
        || constants.fraction_numerator_display_gap_min < 0
        || constants.fraction_rule_thickness <= 0
        || constants.fraction_denominator_gap_min < 0
        || constants.fraction_denominator_display_gap_min < 0
        || constants.radical_vertical_gap < 0
        || constants.radical_display_vertical_gap < 0
        || constants.radical_rule_thickness <= 0
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    Ok(MathTableInfo {
        constants,
        glyph_info_offset,
        variants_offset,
    })
}

fn validate_math_glyph_info_kernel<B: Budget>(
    table: &[u8],
    glyph_info: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(Stage::Header, None)?;
    let italics = usize::from(
        state
            .u16(table, glyph_info)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if italics != 0 {
        validate_math_value_map_kernel(
            table,
            state.relative(glyph_info, italics, table.len())?,
            glyph_count,
            state,
        )?;
    }
    let top_accents = usize::from(
        state
            .u16(table, glyph_info + 2)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if top_accents != 0 {
        validate_math_value_map_kernel(
            table,
            state.relative(glyph_info, top_accents, table.len())?,
            glyph_count,
            state,
        )?;
    }
    let extended_shapes = usize::from(
        state
            .u16(table, glyph_info + 4)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if extended_shapes != 0 {
        validate_coverage_kernel(
            table,
            state.relative(glyph_info, extended_shapes, table.len())?,
            glyph_count,
            state,
        )?;
    }
    if state.u16(table, glyph_info + 6)? != Some(0) {
        return Err(MathFontError::UnsupportedMathGlyphKerning.into());
    }
    Ok(())
}

fn validate_math_value_map_kernel<B: Budget>(
    table: &[u8],
    offset: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(Stage::Header, None)?;
    let coverage_offset = usize::from(
        state
            .u16(table, offset)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let count = usize::from(
        state
            .u16(table, offset + 2)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if count > usize::from(glyph_count)
        || offset
            .checked_add(4)
            .and_then(|value| value.checked_add(count.checked_mul(4)?))
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    let coverage = state.relative(offset, coverage_offset, table.len())?;
    if validate_coverage_kernel(table, coverage, glyph_count, state)? != count {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    for index in 0..count {
        if state.u16(table, offset + 4 + index * 4 + 2)? != Some(0) {
            return Err(MathFontError::UnsupportedMathDeviceTable.into());
        }
    }
    Ok(())
}

fn validate_math_variants_kernel<B: Budget>(
    table: &[u8],
    variants: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(Stage::Header, None)?;
    let vertical_coverage_offset = usize::from(
        state
            .u16(table, variants + 2)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let horizontal_coverage_offset = usize::from(
        state
            .u16(table, variants + 4)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let vertical_count = usize::from(
        state
            .u16(table, variants + 6)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let horizontal_count = usize::from(
        state
            .u16(table, variants + 8)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let total = vertical_count
        .checked_add(horizontal_count)
        .ok_or(MathFontError::MalformedTable("MATH"))?;
    if total > usize::from(glyph_count)
        || variants
            .checked_add(10)
            .and_then(|value| value.checked_add(total.checked_mul(2)?))
            .map_or(true, |end| end > table.len())
        || (vertical_count == 0) != (vertical_coverage_offset == 0)
        || (horizontal_count == 0) != (horizontal_coverage_offset == 0)
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    if vertical_count != 0 {
        let coverage = state.relative(variants, vertical_coverage_offset, table.len())?;
        if validate_coverage_kernel(table, coverage, glyph_count, state)? != vertical_count {
            return Err(MathFontError::MalformedTable("MATH").into());
        }
    }
    if horizontal_count != 0 {
        let coverage = state.relative(variants, horizontal_coverage_offset, table.len())?;
        if validate_coverage_kernel(table, coverage, glyph_count, state)? != horizontal_count {
            return Err(MathFontError::MalformedTable("MATH").into());
        }
    }
    for index in 0..total {
        let construction_offset = usize::from(
            state
                .u16(table, variants + 10 + index * 2)?
                .ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        if construction_offset == 0 {
            return Err(MathFontError::MalformedTable("MATH").into());
        }
        validate_glyph_construction_kernel(
            table,
            state.relative(variants, construction_offset, table.len())?,
            glyph_count,
            state,
        )?;
    }
    Ok(())
}

fn validate_glyph_construction_kernel<B: Budget>(
    table: &[u8],
    construction: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(Stage::Header, None)?;
    let assembly_offset = usize::from(
        state
            .u16(table, construction)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    let count = usize::from(
        state
            .u16(table, construction + 2)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if count > usize::from(glyph_count)
        || (count == 0 && assembly_offset == 0)
        || construction
            .checked_add(4)
            .and_then(|value| value.checked_add(count.checked_mul(4)?))
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    let mut previous_advance = 0u16;
    for index in 0..count {
        let record = construction + 4 + index * 4;
        let glyph = state
            .u16(table, record)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        let advance = state
            .u16(table, record + 2)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        let out_of_order = index != 0
            && match state.variant_order {
                VariantOrder::LegacyStrict => advance <= previous_advance,
                VariantOrder::CffV2Nondecreasing => advance < previous_advance,
            };
        if glyph >= glyph_count || advance == 0 || out_of_order {
            return Err(MathFontError::MalformedTable("MATH").into());
        }
        previous_advance = advance;
    }
    if assembly_offset != 0 {
        validate_glyph_assembly_kernel(
            table,
            state.relative(construction, assembly_offset, table.len())?,
            glyph_count,
            state,
        )?;
    }
    Ok(())
}

fn validate_glyph_assembly_kernel<B: Budget>(
    table: &[u8],
    assembly: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<(), B::Error> {
    state.work(Stage::Header, None)?;
    if state.u16(table, assembly + 2)? != Some(0) {
        return Err(MathFontError::UnsupportedMathDeviceTable.into());
    }
    let count = usize::from(
        state
            .u16(table, assembly + 4)?
            .ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if count == 0
        || count > usize::from(glyph_count)
        || assembly
            .checked_add(6)
            .and_then(|value| value.checked_add(count.checked_mul(10)?))
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH").into());
    }
    for index in 0..count {
        let record = assembly + 6 + index * 10;
        let glyph = state
            .u16(table, record)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        let start = state
            .u16(table, record + 2)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        let end = state
            .u16(table, record + 4)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        let full = state
            .u16(table, record + 6)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        let flags = state
            .u16(table, record + 8)?
            .ok_or(MathFontError::MalformedTable("MATH"))?;
        if glyph >= glyph_count || full == 0 || start > full || end > full || flags & !1 != 0 {
            return Err(MathFontError::MalformedTable("MATH").into());
        }
    }
    Ok(())
}

fn validate_coverage_kernel<B: Budget>(
    table: &[u8],
    coverage: usize,
    glyph_count: u16,
    state: &mut State<'_, B>,
) -> Result<usize, B::Error> {
    state.work(Stage::Header, None)?;
    match state.u16(table, coverage)? {
        Some(1) => {
            let count = usize::from(
                state
                    .u16(table, coverage + 2)?
                    .ok_or(MathFontError::MalformedTable("MATH"))?,
            );
            if count > usize::from(glyph_count)
                || coverage
                    .checked_add(4)
                    .and_then(|value| value.checked_add(count.checked_mul(2)?))
                    .map_or(true, |end| end > table.len())
            {
                return Err(MathFontError::MalformedTable("MATH").into());
            }
            let mut previous = None;
            for index in 0..count {
                let glyph = state
                    .u16(table, coverage + 4 + index * 2)?
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                if glyph >= glyph_count || previous.is_some_and(|value| glyph <= value) {
                    return Err(MathFontError::MalformedTable("MATH").into());
                }
                previous = Some(glyph);
            }
            Ok(count)
        }
        Some(2) => {
            let ranges = usize::from(
                state
                    .u16(table, coverage + 2)?
                    .ok_or(MathFontError::MalformedTable("MATH"))?,
            );
            if ranges > usize::from(glyph_count)
                || coverage
                    .checked_add(4)
                    .and_then(|value| value.checked_add(ranges.checked_mul(6)?))
                    .map_or(true, |end| end > table.len())
            {
                return Err(MathFontError::MalformedTable("MATH").into());
            }
            let mut total = 0usize;
            let mut previous_end = None;
            for index in 0..ranges {
                let record = coverage + 4 + index * 6;
                let start = state
                    .u16(table, record)?
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                let end = state
                    .u16(table, record + 2)?
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                let start_index = usize::from(
                    state
                        .u16(table, record + 4)?
                        .ok_or(MathFontError::MalformedTable("MATH"))?,
                );
                if start > end
                    || end >= glyph_count
                    || previous_end.is_some_and(|value| start <= value)
                    || start_index != total
                {
                    return Err(MathFontError::MalformedTable("MATH").into());
                }
                total = total
                    .checked_add(usize::from(end - start) + 1)
                    .filter(|value| *value <= usize::from(glyph_count))
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                previous_end = Some(end);
            }
            Ok(total)
        }
        _ => Err(MathFontError::MalformedTable("MATH").into()),
    }
}

#[cfg(test)]
#[path = "math_validation_tests.rs"]
pub(crate) mod tests;
