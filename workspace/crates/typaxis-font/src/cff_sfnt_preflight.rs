//! Shared SFNT grammar; ordinary callers keep the frozen allocation policy.
use super::v2::{
    CffAdmissionBudgetV2, CffSfntBudgetFailureV2, CffSfntStageV2, CffSfntStorageV2,
    CffSfntWorkPointV2,
};
use super::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeFailure {
    Native(Cff1Error),
    Allocation,
}
#[derive(Clone, Copy)]
struct Policy {
    allow_vertical: bool,
    allow_fftm: bool,
}
impl Policy {
    fn legacy(allow_vertical: bool) -> Self {
        Self {
            allow_vertical,
            allow_fftm: false,
        }
    }
}
enum Failure<E> {
    Native(Cff1Error),
    Allocation,
    Budget(CffSfntBudgetFailureV2<E>),
}
impl<E> From<Cff1Error> for Failure<E> {
    fn from(e: Cff1Error) -> Self {
        Self::Native(e)
    }
}
impl<E> From<CffSfntBudgetFailureV2<E>> for Failure<E> {
    fn from(e: CffSfntBudgetFailureV2<E>) -> Self {
        Self::Budget(e)
    }
}
type BudgetResult<T, E> = Result<Result<T, NativeFailure>, CffSfntBudgetFailureV2<E>>;
pub(super) fn ordinary(
    source: &[u8],
    limits: M4ResourceLimits,
    context: &mut FontFailureContext,
    allow_vertical: bool,
) -> Result<Vec<TableRecord>, Cff1Error> {
    match with_budget(
        source,
        limits,
        context,
        allow_vertical,
        &mut super::v2::admission_budget::UnobservedAdmission,
    ) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(NativeFailure::Native(e))) => Err(e),
        Ok(Err(NativeFailure::Allocation)) => {
            unreachable!("ordinary allocation policy uses the original native errors")
        }
        Err(e) => match e.error {},
    }
}
pub(super) fn with_budget<B: CffAdmissionBudgetV2>(
    source: &[u8],
    limits: M4ResourceLimits,
    context: &mut FontFailureContext,
    allow_vertical: bool,
    budget: &mut B,
) -> BudgetResult<Vec<TableRecord>, B::Error> {
    with_policy(
        source,
        limits,
        context,
        Policy::legacy(allow_vertical),
        budget,
        &mut |rows, count| rows.try_reserve_exact(count).map_err(|_| ()),
    )
}
pub(super) fn with_v2_budget<B: CffAdmissionBudgetV2>(
    source: &[u8],
    limits: M4ResourceLimits,
    context: &mut FontFailureContext,
    budget: &mut B,
) -> BudgetResult<Vec<TableRecord>, B::Error> {
    with_policy(
        source,
        limits,
        context,
        Policy {
            allow_vertical: true,
            allow_fftm: true,
        },
        budget,
        &mut |rows, count| rows.try_reserve_exact(count).map_err(|_| ()),
    )
}
fn with_policy<B: CffAdmissionBudgetV2>(
    source: &[u8],
    limits: M4ResourceLimits,
    context: &mut FontFailureContext,
    policy: Policy,
    budget: &mut B,
    reserve_native: &mut impl FnMut(&mut Vec<TableRecord>, usize) -> Result<(), ()>,
) -> BudgetResult<Vec<TableRecord>, B::Error> {
    match parse(source, limits, context, policy, budget, reserve_native) {
        Ok(v) => Ok(Ok(v)),
        Err(Failure::Native(e)) => Ok(Err(NativeFailure::Native(e))),
        Err(Failure::Allocation) => Ok(Err(NativeFailure::Allocation)),
        Err(Failure::Budget(e)) => Err(e),
    }
}
fn parse<B: CffAdmissionBudgetV2>(
    source: &[u8],
    limits: M4ResourceLimits,
    context: &mut FontFailureContext,
    policy: Policy,
    budget: &mut B,
    reserve_native: &mut impl FnMut(&mut Vec<TableRecord>, usize) -> Result<(), ()>,
) -> Result<Vec<TableRecord>, Failure<B::Error>> {
    pay(
        budget,
        point(CffSfntStageV2::Header, None, Some(0), None),
        1,
    )?;
    context.directory(None, Some(0), FontFailureReason::MalformedFont);
    if source.get(..4) != Some(b"OTTO") {
        return Err(Cff1Error::InvalidSfnt.into());
    }
    context.file_offset = Some(4);
    let count = usize::from(read_u16(source, 4, Cff1Error::InvalidSfnt)?);
    if count == 0
        || u32::try_from(count).map_err(|_| Cff1Error::TableLimit)? > limits.max_font_tables
    {
        context.reason = if count == 0 {
            FontFailureReason::InvalidTableCount
        } else {
            FontFailureReason::BudgetExceeded
        };
        context.observed = Some(count as u64);
        context.limit = Some(u64::from(limits.max_font_tables));
        return Err(Cff1Error::TableLimit.into());
    }
    context.file_offset = None;
    let largest_power = 1usize << (usize::BITS - 1 - count.leading_zeros());
    let expected_search_range = largest_power
        .checked_mul(16)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(Cff1Error::InvalidSfnt)?;
    let expected_entry_selector =
        u16::try_from(largest_power.trailing_zeros()).map_err(|_| Cff1Error::InvalidSfnt)?;
    let expected_range_shift = count
        .checked_mul(16)
        .and_then(|value| value.checked_sub(usize::from(expected_search_range)))
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(Cff1Error::InvalidSfnt)?;
    if read_u16(source, 6, Cff1Error::InvalidSfnt)? != expected_search_range
        || read_u16(source, 8, Cff1Error::InvalidSfnt)? != expected_entry_selector
        || read_u16(source, 10, Cff1Error::InvalidSfnt)? != expected_range_shift
    {
        return Err(Cff1Error::InvalidSfnt.into());
    }
    let directory_end = 12usize
        .checked_add(count.checked_mul(16).ok_or(Cff1Error::InvalidSfnt)?)
        .ok_or(Cff1Error::InvalidSfnt)?;
    if directory_end > source.len() {
        return Err(Cff1Error::InvalidSfnt.into());
    }
    let mut records = Vec::new();
    reserve_rows(
        budget,
        CffSfntStorageV2::DirectoryRecords,
        count,
        point(CffSfntStageV2::Directory, None, Some(4), None),
    )?;
    if reserve_native(&mut records, count).is_err() {
        return Err(if B::OBSERVED && B::OBSERVE_SFNT {
            Failure::Allocation
        } else {
            Failure::Native(Cff1Error::TableLimit)
        });
    }
    let mut previous_tag = None;
    for index in 0..count {
        let record = 12 + index * 16;
        pay(
            budget,
            point(CffSfntStageV2::Directory, None, Some(record), None),
            1,
        )?;
        let tag: [u8; 4] = source[record..record + 4]
            .try_into()
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        context.directory(
            Some(tag),
            Some(record),
            FontFailureReason::InvalidTableOrder,
        );
        if previous_tag.is_some_and(|previous| previous >= tag) {
            return Err(Cff1Error::InvalidSfnt.into());
        }
        previous_tag = Some(tag);
        if !REQUIRED_TABLES.contains(&tag)
            && !OPTIONAL_TABLES.contains(&tag)
            && !(policy.allow_vertical && [*b"VORG", *b"vhea", *b"vmtx"].contains(&tag))
            && !(policy.allow_fftm && tag == *b"FFTM")
        {
            context.reason = FontFailureReason::UnsupportedTable;
            return Err(Cff1Error::UnsupportedTable.into());
        }
        context.reason = FontFailureReason::InvalidTableRange;
        context.file_offset = Some((record + 8) as u64);
        let expected_checksum = read_u32(source, record + 4, Cff1Error::InvalidSfnt)?;
        let offset = usize::try_from(read_u32(source, record + 8, Cff1Error::InvalidSfnt)?)
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        let length = usize::try_from(read_u32(source, record + 12, Cff1Error::InvalidSfnt)?)
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        let end = offset.checked_add(length).ok_or(Cff1Error::InvalidSfnt)?;
        let padded_end = end
            .checked_add(3)
            .map(|value| value & !3)
            .ok_or(Cff1Error::InvalidSfnt)?;
        if length == 0
            || offset < directory_end
            || offset % 4 != 0
            || end > source.len()
            || padded_end > source.len()
        {
            return Err(Cff1Error::InvalidSfnt.into());
        }
        let actual_checksum = if B::OBSERVED && B::OBSERVE_SFNT {
            if tag == *b"head" && length < 12 {
                return Err(Cff1Error::InvalidHead.into());
            }
            context.reason = FontFailureReason::ChecksumMismatch;
            context.file_offset = Some((record + 4) as u64);
            checksum_observed(
                &source[offset..end],
                Some(tag),
                offset,
                tag == *b"head",
                CffSfntStageV2::TableChecksum,
                budget,
            )?
        } else {
            let mut checksum_bytes = Vec::new();
            checksum_bytes
                .try_reserve_exact(end - offset)
                .map_err(|_| Cff1Error::InvalidSfnt)?;
            checksum_bytes.extend_from_slice(&source[offset..end]);
            if tag == *b"head" {
                if checksum_bytes.len() < 12 {
                    return Err(Cff1Error::InvalidHead.into());
                }
                checksum_bytes[8..12].fill(0);
            }
            context.reason = FontFailureReason::ChecksumMismatch;
            context.file_offset = Some((record + 4) as u64);
            sfnt_checksum(&checksum_bytes)
        };
        if actual_checksum != expected_checksum {
            return Err(Cff1Error::InvalidSfnt.into());
        }
        if tag == *b"FFTM" {
            // Version 1 is exactly u32 + three opaque 64-bit timestamps.
            // Checksums, ranges and padding still use the shared SFNT grammar.
            pay(
                budget,
                point(
                    CffSfntStageV2::FontForgeMetadata,
                    Some(tag),
                    Some(offset),
                    Some(0),
                ),
                32,
            )?;
            let row = TableRecord {
                tag,
                offset,
                length,
                padded_end,
            };
            context.table(
                tag,
                std::slice::from_ref(&row),
                FontFailurePhase::TableDecode,
            );
            context.field(0);
            if length != 28 || source[offset..offset + 4] != [0, 0, 0, 1] {
                return Err(Cff1Error::InvalidOptionalTable.into());
            }
        }
        if tag == *b"OS/2" {
            if policy.allow_fftm {
                context.permission_v2(&source[offset..end]);
            } else {
                context.permission(&source[offset..end]);
            }
        }
        records.push(TableRecord {
            tag,
            offset,
            length,
            padded_end,
        });
    }
    for required in REQUIRED_TABLES {
        pay(
            budget,
            point(CffSfntStageV2::RequiredTables, Some(required), None, None),
            1,
        )?;
        let mut found = false;
        for (index, record) in records.iter().enumerate() {
            pay(
                budget,
                point(
                    CffSfntStageV2::RequiredTables,
                    Some(required),
                    Some(12 + index * 16),
                    None,
                ),
                1,
            )?;
            if record.tag == required {
                found = true;
                break;
            }
        }
        if !found {
            context.directory(Some(required), None, FontFailureReason::MissingTable);
            return Err(Cff1Error::InvalidSfnt.into());
        }
    }
    let by_offset = if B::OBSERVED && B::OBSERVE_SFNT {
        reserve_rows(
            budget,
            CffSfntStorageV2::OrderedRecords,
            records.len(),
            point(CffSfntStageV2::DirectoryCopy, None, None, None),
        )?;
        let mut ordered = Vec::new();
        reserve_native(&mut ordered, records.len()).map_err(|_| Failure::Allocation)?;
        for record in &records {
            pay(
                budget,
                point(CffSfntStageV2::DirectoryCopy, Some(record.tag), None, None),
                1,
            )?;
            ordered.push(*record);
        }
        sort_records(&mut ordered, budget)?;
        ordered
    } else {
        let mut ordered = records.clone();
        ordered.sort_by_key(|record| record.offset);
        ordered
    };
    let mut cursor = directory_end;
    for record in by_offset {
        context.directory(
            Some(record.tag),
            Some(record.offset),
            FontFailureReason::InvalidTableRange,
        );
        // The unit starts here; a bad gap/padding byte can be elsewhere.
        context.position_is_exact = false;
        pay(
            budget,
            point(CffSfntStageV2::Padding, Some(record.tag), None, None),
            1,
        )?;
        if record.offset < cursor
            || !zero_padding(
                &source[cursor..record.offset],
                cursor,
                Some(record.tag),
                None,
                budget,
            )?
        {
            return Err(Cff1Error::InvalidSfnt.into());
        }
        let end = record
            .offset
            .checked_add(record.length)
            .ok_or(Cff1Error::InvalidSfnt)?;
        if !zero_padding(
            &source[end..record.padded_end],
            end,
            Some(record.tag),
            Some(record.length),
            budget,
        )? {
            return Err(Cff1Error::InvalidSfnt.into());
        }
        cursor = record.padded_end;
    }
    context.directory(None, None, FontFailureReason::ChecksumMismatch);
    if cursor != source.len()
        || (if B::OBSERVED && B::OBSERVE_SFNT {
            checksum_observed(source, None, 0, false, CffSfntStageV2::FontChecksum, budget)?
        } else {
            sfnt_checksum(source)
        }) != SFNT_CHECKSUM_MAGIC
    {
        return Err(Cff1Error::InvalidSfnt.into());
    }
    Ok(records)
}

fn point(
    stage: CffSfntStageV2,
    table_tag: Option<[u8; 4]>,
    file: Option<usize>,
    table: Option<usize>,
) -> CffSfntWorkPointV2 {
    CffSfntWorkPointV2 {
        stage,
        table_tag,
        file_offset: file.map(|n| n as u64),
        table_offset: table.map(|n| n as u64),
    }
}
fn pay<B: CffAdmissionBudgetV2>(
    budget: &mut B,
    point: CffSfntWorkPointV2,
    n: u64,
) -> Result<(), CffSfntBudgetFailureV2<B::Error>> {
    if B::OBSERVED && B::OBSERVE_SFNT {
        budget
            .work_sfnt(n)
            .map_err(|error| CffSfntBudgetFailureV2 { error, point })?;
    }
    Ok(())
}
fn reserve_rows<B: CffAdmissionBudgetV2>(
    budget: &mut B,
    site: CffSfntStorageV2,
    rows: usize,
    point: CffSfntWorkPointV2,
) -> Result<(), Failure<B::Error>> {
    if B::OBSERVED && B::OBSERVE_SFNT {
        let bytes = rows
            .checked_mul(std::mem::size_of::<TableRecord>())
            .ok_or(Failure::Allocation)?;
        budget
            .reserve_sfnt(site, rows, bytes)
            .map_err(|error| CffSfntBudgetFailureV2 { error, point })?;
    }
    Ok(())
}
fn checksum_observed<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    tag: Option<[u8; 4]>,
    base: usize,
    head: bool,
    stage: CffSfntStageV2,
    budget: &mut B,
) -> Result<u32, CffSfntBudgetFailureV2<B::Error>> {
    let mut sum = 0u32;
    for (batch, chunk) in bytes.chunks(256).enumerate() {
        let at = batch * 256;
        pay(
            budget,
            point(stage, tag, Some(base + at), tag.map(|_| at)),
            chunk.len().div_ceil(4) as u64,
        )?;
        for (i, word) in chunk.chunks(4).enumerate() {
            let mut buffer = [0u8; 4];
            if !(head && at + i * 4 == 8) {
                buffer[..word.len()].copy_from_slice(word);
            }
            sum = sum.wrapping_add(u32::from_be_bytes(buffer));
        }
    }
    Ok(sum)
}
fn zero_padding<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    base: usize,
    tag: Option<[u8; 4]>,
    relative: Option<usize>,
    budget: &mut B,
) -> Result<bool, CffSfntBudgetFailureV2<B::Error>> {
    if !B::OBSERVED || !B::OBSERVE_SFNT {
        return Ok(!bytes.iter().any(|b| *b != 0));
    }
    for (i, chunk) in bytes.chunks(256).enumerate() {
        let at = i * 256;
        pay(
            budget,
            point(
                CffSfntStageV2::Padding,
                tag,
                Some(base + at),
                relative.map(|n| n + at),
            ),
            chunk.len() as u64,
        )?;
        if chunk.iter().any(|b| *b != 0) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn sort_records<B: CffAdmissionBudgetV2>(
    rows: &mut [TableRecord],
    budget: &mut B,
) -> Result<(), CffSfntBudgetFailureV2<B::Error>> {
    fn key(r: &TableRecord) -> (usize, [u8; 4]) {
        (r.offset, r.tag)
    }
    fn sift<B: CffAdmissionBudgetV2>(
        a: &mut [TableRecord],
        mut root: usize,
        end: usize,
        b: &mut B,
    ) -> Result<(), CffSfntBudgetFailureV2<B::Error>> {
        while root < end / 2 {
            let mut child = 2 * root + 1;
            if child + 1 < end {
                pay(
                    b,
                    point(
                        CffSfntStageV2::DirectoryOrder,
                        Some(a[child].tag),
                        None,
                        None,
                    ),
                    1,
                )?;
                if key(&a[child]) < key(&a[child + 1]) {
                    child += 1;
                }
            }
            pay(
                b,
                point(
                    CffSfntStageV2::DirectoryOrder,
                    Some(a[root].tag),
                    None,
                    None,
                ),
                1,
            )?;
            if key(&a[root]) >= key(&a[child]) {
                break;
            }
            pay(
                b,
                point(
                    CffSfntStageV2::DirectoryOrder,
                    Some(a[root].tag),
                    None,
                    None,
                ),
                1,
            )?;
            a.swap(root, child);
            root = child;
        }
        Ok(())
    }
    pay(
        budget,
        point(CffSfntStageV2::DirectoryOrder, None, None, None),
        1,
    )?;
    for root in (0..rows.len() / 2).rev() {
        sift(rows, root, rows.len(), budget)?;
    }
    for end in (1..rows.len()).rev() {
        pay(
            budget,
            point(
                CffSfntStageV2::DirectoryOrder,
                Some(rows[0].tag),
                None,
                None,
            ),
            1,
        )?;
        rows.swap(0, end);
        sift(rows, 0, end, budget)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "cff_sfnt_preflight_tests.rs"]
mod tests;
