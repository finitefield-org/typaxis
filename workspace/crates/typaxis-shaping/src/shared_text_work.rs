//! Fallible visits in itemization and run selection. The ordinary helpers keep
//! their pinned library operations; observed helpers borrow the same input.
use super::*;
use crate::shared_shape_storage::Storage;

pub(super) fn script_candidates<S: Storage>(
    text: &str,
    storage: &mut S,
) -> Result<ScriptExtension, LinkedShaperError> {
    if !S::OBSERVED {
        return Ok(ScriptExtension::for_str(text));
    }
    let mut result = ScriptExtension::default();
    let mut characters = text.chars();
    loop {
        storage.work(1)?;
        let Some(character) = characters.next() else {
            break;
        };
        result.intersect_with(character.into());
    }
    Ok(result)
}

pub(super) fn mixed_levels<S: Storage>(
    levels: &[unicode_bidi::Level],
    expected: unicode_bidi::Level,
    storage: &mut S,
) -> Result<bool, LinkedShaperError> {
    if !S::OBSERVED {
        return Ok(levels.iter().any(|candidate| *candidate != expected));
    }
    for candidate in levels {
        storage.work(1)?;
        if *candidate != expected {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn line_index<S: Storage>(
    ends: &[u32],
    start: usize,
    storage: &mut S,
) -> Result<usize, LinkedShaperError> {
    if !S::OBSERVED {
        return Ok(ends.partition_point(|end| *end as usize <= start));
    }
    storage.work(1)?;
    let (mut left, mut right) = (0, ends.len());
    while left < right {
        storage.work(1)?;
        let middle = left + (right - left) / 2;
        if ends[middle] as usize <= start {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    Ok(left)
}

pub(super) fn number_width<S: Storage>(
    runs: &[StagingEquationNumberGlyphRun],
    storage: &mut S,
) -> Result<Option<PositiveLength>, LinkedShaperError> {
    if !S::OBSERVED {
        return Ok(equation_number_runs_width(runs));
    }
    let mut raw = 0i64;
    for run in runs {
        storage.work(1)?;
        for glyph in &run.glyphs {
            storage.work(1)?;
            let Some(next) = raw.checked_add(glyph.advance_x.raw()) else {
                return Ok(None);
            };
            raw = next;
        }
    }
    Ok(Length::from_raw(raw).and_then(PositiveLength::new))
}

#[cfg(test)]
#[path = "shared_text_work_tests.rs"]
mod tests;
