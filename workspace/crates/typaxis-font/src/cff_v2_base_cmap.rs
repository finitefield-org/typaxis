//! Direct ordered CFF /2 base mappings. Every vector allocation is fallible;
//! observed requests cover this base map; other admission phases and allocator
//! overcapacity remain independent.
use super::super::super::cmap_visitors::{visit_format12, visit_format4};
use super::super::base_cmap_budget::{
    CffBaseCmapBudgetFailureV2, CffBaseCmapBudgetResultV2, CffBaseCmapBudgetV2,
    CffBaseCmapStorageV2,
};
use super::*;

#[cfg(test)]
use super::super::base_cmap_budget::NoBaseCmapBudget;

type Mapping = (u32, u16);
#[derive(Debug)]
enum Failure<E> {
    Base(Cff1Error),
    Storage,
    Budget(E),
}

fn storage_failure<E>() -> Failure<E> {
    Failure::Storage
}
impl<E> From<Cff1Error> for Failure<E> {
    fn from(kind: Cff1Error) -> Self {
        Self::Base(kind)
    }
}
fn reserve_rows(rows: &mut Vec<Mapping>, count: usize) -> Result<(), ()> {
    rows.try_reserve_exact(count).map_err(|_| ())
}

#[cfg(test)]
fn ordinary<T>(
    value: CffBaseCmapBudgetResultV2<T, CffCmapFailureV2, std::convert::Infallible>,
) -> Result<T, CffCmapFailureV2> {
    match value {
        Ok(value) => value,
        Err(failure) => match failure.error {},
    }
}
#[cfg(test)]
pub(super) fn parse(
    bytes: &[u8],
    glyph_count: u16,
    context: &mut FontFailureContext,
    validated_variations: bool,
) -> Result<Vec<Mapping>, CffCmapFailureV2> {
    ordinary(parse_with_budget(
        bytes,
        glyph_count,
        context,
        validated_variations,
        &mut NoBaseCmapBudget,
    ))
}
pub(super) fn parse_with_budget<B: CffBaseCmapBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    context: &mut FontFailureContext,
    validated_variations: bool,
    budget: &mut B,
) -> CffBaseCmapBudgetResultV2<Vec<Mapping>, CffCmapFailureV2, B::Error> {
    parse_with_policy(
        bytes,
        glyph_count,
        context,
        validated_variations,
        budget,
        &mut reserve_rows,
    )
}
#[cfg(test)]
fn parse_with_reserve(
    bytes: &[u8],
    glyph_count: u16,
    context: &mut FontFailureContext,
    validated_variations: bool,
    reserve: &mut impl FnMut(&mut Vec<Mapping>, usize) -> Result<(), ()>,
) -> Result<Vec<Mapping>, CffCmapFailureV2> {
    ordinary(parse_with_policy(
        bytes,
        glyph_count,
        context,
        validated_variations,
        &mut NoBaseCmapBudget,
        reserve,
    ))
}
fn parse_with_policy<B: CffBaseCmapBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    context: &mut FontFailureContext,
    validated_variations: bool,
    budget: &mut B,
    reserve: &mut impl FnMut(&mut Vec<Mapping>, usize) -> Result<(), ()>,
) -> CffBaseCmapBudgetResultV2<Vec<Mapping>, CffCmapFailureV2, B::Error> {
    let mut parse = || -> Result<Vec<Mapping>, Failure<B::Error>> {
        work(budget, 1)?;
        context.field(0);
        if read_u16(bytes, 0, Cff1Error::InvalidCmap)? != 0 {
            return Err(Cff1Error::InvalidCmap.into());
        }
        context.field(2);
        let count = usize::from(read_u16(bytes, 2, Cff1Error::InvalidCmap)?);
        if count == 0 {
            return Err(Cff1Error::InvalidCmap.into());
        }
        let records_end = 4usize
            .checked_add(count.checked_mul(8).ok_or(Cff1Error::InvalidCmap)?)
            .ok_or(Cff1Error::InvalidCmap)?;
        if records_end > bytes.len() {
            return Err(Cff1Error::InvalidCmap.into());
        }
        let mut merged = Vec::new();
        let mut has_base = false;
        for index in 0..count {
            work(budget, 1)?;
            let record = 4 + index * 8;
            context.cmap_format = None;
            context.field(record);
            let platform = read_u16(bytes, record, Cff1Error::InvalidCmap)?;
            let encoding = read_u16(bytes, record + 2, Cff1Error::InvalidCmap)?;
            if !matches!((platform, encoding), (0, _) | (1, 0) | (3, 1) | (3, 10)) {
                return Err(Cff1Error::InvalidCmap.into());
            }
            context.field(record + 4);
            let offset = usize::try_from(read_u32(bytes, record + 4, Cff1Error::InvalidCmap)?)
                .map_err(|_| Cff1Error::InvalidCmap)?;
            if offset < records_end {
                return Err(Cff1Error::InvalidCmap.into());
            }
            context.field(offset);
            let format = read_u16(bytes, offset, Cff1Error::InvalidCmap)?;
            context.cmap_format = Some(format);
            if validated_variations && (platform, encoding, format) == (0, 5, 14) {
                continue;
            }
            if (platform, encoding, format) == (1, 0, 6) {
                // A legacy Mac Roman subtable is structural metadata only.
                // It never supplies or repairs the required Unicode base map.
                validate_mac_roman6(bytes, offset, glyph_count, context, budget)?;
                continue;
            }
            let valid = match format {
                4 => platform == 0 || (platform == 3 && encoding == 1),
                12 => platform == 0 || (platform == 3 && encoding == 10),
                _ => false,
            };
            if !valid {
                context.reason = FontFailureReason::UnsupportedCmapFormat;
                return Err(Cff1Error::InvalidCmap.into());
            }
            context.at(offset);
            let rows = read_rows(bytes, offset, glyph_count, format, budget, reserve)?;
            has_base = true;
            context.clear_position();
            merged = merge_with_budget(merged, rows, budget, reserve)?;
        }
        if !has_base || (merged.is_empty() && !validated_variations) {
            return Err(Cff1Error::InvalidCmap.into());
        }
        Ok(merged)
    };
    match parse() {
        Ok(rows) => Ok(Ok(rows)),
        Err(Failure::Base(kind)) => Ok(Err(CffCmapFailureV2::Base {
            kind,
            context: *context,
        })),
        Err(Failure::Storage) => Ok(Err(CffCmapFailureV2::Storage(CffTableFailureV2 {
            table: *b"cmap",
            table_offset: 0,
            kind: CffTableFailureKindV2::AllocationFailure,
            limit: None,
            observed: None,
        }))),
        Err(Failure::Budget(error)) => Err(CffBaseCmapBudgetFailureV2 {
            error,
            table_offset: context.table_offset,
            file_offset: None,
            cmap_format: context.cmap_format,
        }),
    }
}
fn validate_mac_roman6<B: CffBaseCmapBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    context: &mut FontFailureContext,
    budget: &mut B,
) -> Result<(), Failure<B::Error>> {
    work(budget, 1)?;
    context.field(offset + 2);
    let length = usize::from(read_u16(bytes, offset + 2, Cff1Error::InvalidCmap)?);
    context.field(offset + 4);
    if read_u16(bytes, offset + 4, Cff1Error::InvalidCmap)? != 0 {
        return Err(Cff1Error::InvalidCmap.into());
    }
    context.field(offset + 6);
    let first = usize::from(read_u16(bytes, offset + 6, Cff1Error::InvalidCmap)?);
    context.field(offset + 8);
    let count = usize::from(read_u16(bytes, offset + 8, Cff1Error::InvalidCmap)?);
    if first + count > 256 || length != 10 + count * 2 || offset + length > bytes.len() {
        return Err(Cff1Error::InvalidCmap.into());
    }
    for index in 0..count {
        work(budget, 1)?;
        context.field(offset + 10 + index * 2);
        if read_u16(bytes, offset + 10 + index * 2, Cff1Error::InvalidCmap)? >= glyph_count {
            return Err(Cff1Error::InvalidCmap.into());
        }
    }
    Ok(())
}
fn work<B: CffBaseCmapBudgetV2>(budget: &mut B, n: u64) -> Result<(), Failure<B::Error>> {
    if B::OBSERVED {
        budget.work(n).map_err(Failure::Budget)?;
    }
    Ok(())
}
fn reserve_mappings<B: CffBaseCmapBudgetV2>(
    budget: &mut B,
    site: CffBaseCmapStorageV2,
    count: usize,
) -> Result<(), Failure<B::Error>> {
    if B::OBSERVED {
        let bytes = count
            .checked_mul(std::mem::size_of::<Mapping>())
            .ok_or_else(storage_failure)?;
        budget
            .reserve(site, count, bytes)
            .map_err(Failure::Budget)?;
    }
    Ok(())
}

fn visit<E: From<Cff1Error>>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    format: u16,
    emit: impl FnMut(u32, u16) -> Result<(), E>,
    work: &mut impl FnMut(u64) -> Result<(), E>,
) -> Result<(), E> {
    match format {
        4 => visit_format4(bytes, offset, glyph_count, emit, work),
        12 => visit_format12(bytes, offset, glyph_count, emit, work),
        _ => unreachable!("the record format was validated before its decoder"),
    }
}

fn read_rows<B: CffBaseCmapBudgetV2>(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    format: u16,
    budget: &mut B,
    reserve: &mut impl FnMut(&mut Vec<Mapping>, usize) -> Result<(), ()>,
) -> Result<Vec<Mapping>, Failure<B::Error>> {
    let mut count = 0usize;
    visit(
        bytes,
        offset,
        glyph_count,
        format,
        |_, _| {
            count = count.checked_add(1).ok_or_else(storage_failure)?;
            Ok::<_, Failure<B::Error>>(())
        },
        &mut |n| work(budget, n),
    )?;
    let mut rows = Vec::new();
    if count > 0 {
        reserve_mappings(budget, CffBaseCmapStorageV2::SubtableMappings, count)?;
        reserve(&mut rows, count).map_err(|_| storage_failure())?;
        visit(
            bytes,
            offset,
            glyph_count,
            format,
            |scalar, gid| {
                rows.push((scalar, gid));
                Ok::<_, Failure<B::Error>>(())
            },
            &mut |n| work(budget, n),
        )?;
    }
    Ok(rows)
}

fn merge_with_budget<B: CffBaseCmapBudgetV2>(
    left: Vec<Mapping>,
    right: Vec<Mapping>,
    budget: &mut B,
    reserve: &mut impl FnMut(&mut Vec<Mapping>, usize) -> Result<(), ()>,
) -> Result<Vec<Mapping>, Failure<B::Error>> {
    work(budget, 1)?;
    if left.is_empty() {
        return Ok(right);
    }
    if right.is_empty() {
        return Ok(left);
    }
    let maximum = left
        .len()
        .checked_add(right.len())
        .ok_or_else(storage_failure)?;
    let mut count = 0usize;
    merge_visit_with_budget(&left, &right, budget, |_| {
        count += 1;
    })?;
    debug_assert!(count <= maximum);
    // Already validated aliases/subsets keep an existing allocation.
    if count == left.len() {
        return Ok(left);
    }
    if count == right.len() {
        return Ok(right);
    }
    let mut rows = Vec::new();
    reserve_mappings(budget, CffBaseCmapStorageV2::MergedMappings, count)?;
    reserve(&mut rows, count).map_err(|_| storage_failure())?;
    merge_visit_with_budget(&left, &right, budget, |row| rows.push(row))?;
    Ok(rows)
}

fn merge_visit_with_budget<B: CffBaseCmapBudgetV2>(
    left: &[Mapping],
    right: &[Mapping],
    budget: &mut B,
    mut emit: impl FnMut(Mapping),
) -> Result<(), Failure<B::Error>> {
    let (mut a, mut b) = (0, 0);
    while a < left.len() && b < right.len() {
        work(budget, 1)?;
        match left[a].0.cmp(&right[b].0) {
            std::cmp::Ordering::Less => {
                emit(left[a]);
                a += 1;
            }
            std::cmp::Ordering::Greater => {
                emit(right[b]);
                b += 1;
            }
            std::cmp::Ordering::Equal => {
                if left[a].1 != right[b].1 {
                    return Err(Cff1Error::InvalidCmap.into());
                }
                emit(left[a]);
                a += 1;
                b += 1;
            }
        }
    }
    for &row in &left[a..] {
        work(budget, 1)?;
        emit(row);
    }
    for &row in &right[b..] {
        work(budget, 1)?;
        emit(row);
    }
    Ok(())
}

#[cfg(test)]
fn merge(
    left: Vec<Mapping>,
    right: Vec<Mapping>,
    reserve: &mut impl FnMut(&mut Vec<Mapping>, usize) -> Result<(), ()>,
) -> Result<Vec<Mapping>, Failure<std::convert::Infallible>> {
    merge_with_budget(left, right, &mut NoBaseCmapBudget, reserve)
}

#[cfg(test)]
#[path = "cff_v2_base_cmap_tests.rs"]
mod tests;
