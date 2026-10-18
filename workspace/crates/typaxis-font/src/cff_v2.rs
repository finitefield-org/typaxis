//! CFF /2 program structure. This is not sfnt admission or PDF authorization.
//! Byte ranges retain the original CFF table once; no per-program byte copies.
use super::*;
use std::sync::Arc;
use CffProgramStageV2 as PStage;
use CffProgramStorageV2 as PSite;
#[path = "cff_v2_source_owner.rs"]
mod source_owner;
use source_owner::SourceBytesV2;
#[path = "cff_v2_name_keyed.rs"]
mod name_keyed;
pub use name_keyed::CffSourceGlyphV2;
use name_keyed::SourceCharset;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffProgramErrorKindV2 {
    InvalidStructure,
    UnsupportedOperator,
    UnsupportedFontMatrix,
    ExpectedCidFont,
    InvalidCharset,
    InvalidFdSelect,
    OverlappingStructures,
    OperandLimit,
    GlyphLimit,
    FontDictLimit,
    SubroutineLimit,
    AllocationFailure,
    InvalidEncoding,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffProgramErrorV2 {
    pub kind: CffProgramErrorKindV2,
    pub table_offset: usize,
    pub fd: Option<u16>,
    pub operator: Option<u16>,
    pub limit: Option<u64>,
    pub observed: Option<u64>,
}
impl std::fmt::Display for CffProgramErrorV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 {:?} at {}", self.kind, self.table_offset)
    }
}
impl std::error::Error for CffProgramErrorV2 {}
use CffProgramErrorKindV2 as K;
fn fail(kind: K, offset: usize) -> CffProgramErrorV2 {
    CffProgramErrorV2 {
        kind,
        table_offset: offset,
        fd: None,
        operator: None,
        limit: None,
        observed: None,
    }
}
fn limited(kind: K, offset: usize, limit: u64, observed: u64) -> CffProgramErrorV2 {
    CffProgramErrorV2 {
        limit: Some(limit),
        observed: Some(observed),
        ..fail(kind, offset)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ProgramSpan {
    start: usize,
    end: usize,
}
impl ProgramSpan {
    fn bytes(self, source: &[u8]) -> &[u8] {
        &source[self.start..self.end]
    }
}
#[derive(Debug)]
struct ProgramIndex {
    span: ProgramSpan,
    objects: Vec<ProgramSpan>,
}
fn u16_at(b: &[u8], at: usize) -> Result<u16, CffProgramErrorV2> {
    read_u16(b, at, Cff1Error::InvalidCff).map_err(|_| fail(K::InvalidStructure, at))
}
fn range(b: &[u8], at: usize, count: usize) -> Result<ProgramSpan, CffProgramErrorV2> {
    let end = at
        .checked_add(count)
        .filter(|end| *end <= b.len())
        .ok_or_else(|| fail(K::InvalidStructure, at))?;
    Ok(ProgramSpan { start: at, end })
}
#[cfg(test)]
fn index(b: &[u8], at: usize, maximum: usize, axis: K) -> Result<ProgramIndex, CffProgramErrorV2> {
    let mut budget = admission_budget::UnobservedAdmission;
    let mut state = program_budget::State::new(&mut budget);
    match program_budget::finish(index_observed(b, at, maximum, axis, &mut state)) {
        Ok(native) => native,
        Err(e) => match e.error {},
    }
}
fn index_observed<B: CffAdmissionBudgetV2>(
    b: &[u8],
    at: usize,
    maximum: usize,
    axis: K,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<ProgramIndex, B::Error> {
    s.source_length = Some(b.len());
    s.work(PStage::IndexHeader, Some(at), None, 8)?;
    let count = usize::from(u16_at(b, at)?);
    if count > maximum {
        return Err(limited(axis, at, maximum as u64, count as u64).into());
    }
    if count == 0 {
        return Ok(ProgramIndex {
            span: range(b, at, 2)?,
            objects: Vec::new(),
        });
    }
    let size = usize::from(
        *b.get(at + 2)
            .ok_or_else(|| fail(K::InvalidStructure, at + 2))?,
    );
    if !(1..=4).contains(&size) {
        return Err(fail(K::InvalidStructure, at + 2).into());
    }
    let offsets = range(b, at + 3, (count + 1) * size)?;
    let mut objects = Vec::new();
    s.reserve(&mut objects, count, PSite::IndexObjects, at)?;
    let mut previous = 0usize;
    for i in 0..=count {
        let offset = at + 3 + i * size;
        s.work(PStage::IndexOffset, Some(offset), None, 1)?;
        let value = b[offset..offset + size]
            .iter()
            .try_fold(0usize, |n, byte| {
                n.checked_mul(256)
                    .and_then(|n| n.checked_add(usize::from(*byte)))
            })
            .ok_or_else(|| fail(K::InvalidStructure, offset))?;
        if value == 0 || (i == 0 && value != 1) || value < previous {
            return Err(fail(K::InvalidStructure, offset).into());
        }
        let end = offsets
            .end
            .checked_add(value - 1)
            .filter(|v| *v <= b.len())
            .ok_or_else(|| fail(K::InvalidStructure, offset))?;
        if i > 0 {
            objects.push(ProgramSpan {
                start: offsets.end + previous - 1,
                end,
            });
        }
        previous = value;
    }
    Ok(ProgramIndex {
        span: ProgramSpan {
            start: at,
            end: offsets.end + previous - 1,
        },
        objects,
    })
}
#[cfg(test)]
fn dict(b: &[u8], span: ProgramSpan) -> Result<Vec<DictEntry>, CffProgramErrorV2> {
    let mut budget = admission_budget::UnobservedAdmission;
    let mut state = program_budget::State::new(&mut budget);
    match program_budget::finish(dict_observed(b, span, &mut state)) {
        Ok(native) => native,
        Err(e) => match e.error {},
    }
}
fn dict_observed<B: CffAdmissionBudgetV2>(
    b: &[u8],
    span: ProgramSpan,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<Vec<DictEntry>, B::Error> {
    s.source_length = Some(b.len());
    let bytes = span.bytes(b);
    let mut entries: Vec<DictEntry> = Vec::new();
    let mut operands = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        s.work(PStage::DictToken, Some(span.start + cursor), None, 1)?;
        let value = bytes[cursor];
        if value >= 28 {
            if operands.len() == 48 {
                return Err(limited(K::OperandLimit, span.start + cursor, 48, 49).into());
            }
            let (operand, consumed) = if value == 30 && B::OBSERVE_PROGRAM {
                s.work(PStage::DictNumber, Some(span.start + cursor), None, 1024)?;
                let (real, consumed) = decode_dict_real(bytes, cursor)
                    .map_err(|_| fail(K::InvalidStructure, span.start + cursor))?;
                let mut value = String::new();
                s.string(&mut value, real.length, span.start + cursor)?;
                value.push_str(real.as_str());
                (DictOperand::Real(value), consumed)
            } else {
                s.work(
                    PStage::DictNumber,
                    Some(span.start + cursor),
                    None,
                    if value == 30 { 1024 } else { 8 },
                )?;
                parse_dict_operand(bytes, cursor)
                    .map_err(|_| fail(K::InvalidStructure, span.start + cursor))?
            };
            s.reserve(&mut operands, 1, PSite::DictOperands, span.start + cursor)?;
            operands.push(operand);
            cursor += consumed;
        } else {
            let (operator, consumed) = if value == 12 {
                (
                    0x0c00
                        | u16::from(
                            *bytes
                                .get(cursor + 1)
                                .ok_or_else(|| fail(K::InvalidStructure, span.start + cursor))?,
                        ),
                    2,
                )
            } else if value <= 21 {
                (u16::from(value), 1)
            } else {
                return Err(fail(K::InvalidStructure, span.start + cursor).into());
            };
            for e in &entries {
                s.work(
                    PStage::DictDuplicate,
                    Some(span.start + cursor),
                    Some(operator),
                    1,
                )?;
                if e.operator == operator {
                    return Err(fail(K::InvalidStructure, span.start + cursor).into());
                }
            }
            s.reserve(&mut entries, 1, PSite::DictEntries, span.start + cursor)?;
            entries.push(DictEntry {
                operator_offset: span.start + cursor,
                operator,
                operands: std::mem::take(&mut operands),
            });
            cursor += consumed;
        }
    }
    if !operands.is_empty() {
        return Err(fail(K::InvalidStructure, span.end).into());
    }
    Ok(entries)
}
fn integer_observed<B: CffAdmissionBudgetV2>(
    entries: &[DictEntry],
    op: u16,
    at: usize,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<Option<i32>, B::Error> {
    s.work(
        PStage::DictLookup,
        Some(at),
        Some(op),
        entries.len() as u64 + 1,
    )?;
    Ok(one_dict_integer(entries, op).map_err(|_| fail(K::InvalidStructure, at))?)
}
#[cfg(test)]
fn offset(entries: &[DictEntry], op: u16, at: usize) -> Result<usize, CffProgramErrorV2> {
    let mut budget = admission_budget::UnobservedAdmission;
    let mut state = program_budget::State::new(&mut budget);
    match program_budget::finish(offset_observed(entries, op, at, &mut state)) {
        Ok(native) => native,
        Err(e) => match e.error {},
    }
}
fn offset_observed<B: CffAdmissionBudgetV2>(
    entries: &[DictEntry],
    op: u16,
    at: usize,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<usize, B::Error> {
    Ok(usize::try_from(
        integer_observed(entries, op, at, s)?.ok_or_else(|| fail(K::InvalidStructure, at))?,
    )
    .map_err(|_| fail(K::InvalidStructure, at))?)
}
fn valid_sid(value: &DictOperand, strings: usize) -> bool {
    matches!(value, DictOperand::Integer(sid) if *sid >= 0 && *sid < 65000 && (*sid < 391 || (*sid as usize - 391) < strings))
}
fn matrix(
    entries: &[DictEntry],
    units_per_em: u16,
    font_dict: bool,
    at: usize,
) -> Result<(), CffProgramErrorV2> {
    let entry = entries.iter().find(|e| e.operator == 0x0c07);
    if units_per_em != 1000
        || (font_dict && entry.is_some())
        || entry.is_some_and(|e| {
            let v = &e.operands;
            v.len() != 6
                || !dict_operand_equals_milli(&v[0])
                || !dict_operand_equals_integer(&v[1], 0)
                || !dict_operand_equals_integer(&v[2], 0)
                || !dict_operand_equals_milli(&v[3])
                || !dict_operand_equals_integer(&v[4], 0)
                || !dict_operand_equals_integer(&v[5], 0)
        })
    {
        return Err(fail(
            K::UnsupportedFontMatrix,
            entry.map_or(at, |e| e.operator_offset),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct CffFontDictV2 {
    private_span: ProgramSpan,
    local_subrs: Vec<ProgramSpan>,
    default_width_x: i32,
    nominal_width_x: i32,
}
#[derive(Debug)]
struct CffProgramV2 {
    source: SourceBytesV2,
    source_span: Option<ProgramSpan>,
    charstrings: Vec<ProgramSpan>,
    global_subrs: Vec<ProgramSpan>,
    font_dicts: Vec<CffFontDictV2>,
    fd_by_gid: Vec<u8>,
    charset: SourceCharset,
}
impl CffProgramV2 {
    fn bytes(&self) -> &[u8] {
        self.source_span
            .map_or(&self.source, |span| span.bytes(&self.source))
    }
}
/// Structural inspection only: does not authorize a font resource or execute
/// unselected CharStrings. The subsequent /2 admission owner validates sfnt,
/// embedding, cmap/UVS and horizontal/vertical tables against this program.
#[derive(Debug)]
pub struct CffProgramInspectionV2 {
    program: CffProgramV2,
}
impl CffProgramInspectionV2 {
    pub fn glyph_count(&self) -> usize {
        self.program.charstrings.len()
    }
    pub fn font_dict_count(&self) -> usize {
        self.program.font_dicts.len()
    }
    pub fn global_subroutine_count(&self) -> usize {
        self.program.global_subrs.len()
    }
    pub fn local_subroutine_count(&self, fd: usize) -> Option<usize> {
        self.program.font_dicts.get(fd).map(|d| d.local_subrs.len())
    }
    pub fn fd_for_gid(&self, gid: usize) -> Option<u8> {
        self.program.fd_by_gid.get(gid).copied()
    }
    pub fn cid_for_gid(&self, gid: usize) -> Option<u16> {
        match self.source_glyph(gid) {
            Some(CffSourceGlyphV2::Cid(cid)) => Some(cid),
            _ => None,
        }
    }
    pub fn sid_for_gid(&self, gid: usize) -> Option<u16> {
        match self.source_glyph(gid) {
            Some(CffSourceGlyphV2::Sid(sid)) => Some(sid),
            _ => None,
        }
    }
    pub fn source_glyph(&self, gid: usize) -> Option<CffSourceGlyphV2> {
        self.program.charset.glyph(gid)
    }
    pub fn width_defaults(&self, fd: usize) -> Option<(i32, i32)> {
        self.program
            .font_dicts
            .get(fd)
            .map(|d| (d.default_width_x, d.nominal_width_x))
    }
    pub fn charstring(&self, gid: usize) -> Option<&[u8]> {
        self.program
            .charstrings
            .get(gid)
            .map(|s| s.bytes(self.program.bytes()))
    }
    pub fn private_dict_bytes(&self, fd: usize) -> Option<&[u8]> {
        self.program
            .font_dicts
            .get(fd)
            .map(|d| d.private_span.bytes(self.program.bytes()))
    }
}

/// Parse a CID-keyed or name-keyed CFF1 table for the version-2 implementation. Source is the
/// exact table bytes, not a renamed/stripped font. This is not public admission.
pub fn inspect_cff1_program_v2(
    source: Arc<[u8]>,
    glyph_count: u16,
    units_per_em: u16,
    max_subroutines: u32,
) -> Result<CffProgramInspectionV2, CffProgramErrorV2> {
    inspect_cff1_program_v2_with_metadata(source, glyph_count, units_per_em, max_subroutines, None)
}
fn inspect_cff1_program_v2_with_metadata(
    source: Arc<[u8]>,
    glyph_count: u16,
    units_per_em: u16,
    max_subroutines: u32,
    metadata: Option<(&str, [i16; 4])>,
) -> Result<CffProgramInspectionV2, CffProgramErrorV2> {
    inspect_program_range(
        source,
        None,
        glyph_count,
        units_per_em,
        max_subroutines,
        metadata,
    )
}
fn inspect_program_range(
    source: impl Into<SourceBytesV2>,
    source_span: Option<ProgramSpan>,
    glyph_count: u16,
    units_per_em: u16,
    max_subroutines: u32,
    metadata: Option<(&str, [i16; 4])>,
) -> Result<CffProgramInspectionV2, CffProgramErrorV2> {
    match inspect_program_range_with_budget(
        source,
        source_span,
        glyph_count,
        units_per_em,
        max_subroutines,
        metadata,
        &mut admission_budget::UnobservedAdmission,
    ) {
        Ok(native) => native,
        Err(e) => match e.error {},
    }
}
pub fn inspect_cff1_program_v2_with_budget<B: CffAdmissionBudgetV2>(
    source: Arc<[u8]>,
    glyph_count: u16,
    units_per_em: u16,
    max_subroutines: u32,
    budget: &mut B,
) -> CffProgramBudgetResultV2<CffProgramInspectionV2, B::Error> {
    inspect_program_range_with_budget(
        source,
        None,
        glyph_count,
        units_per_em,
        max_subroutines,
        None,
        budget,
    )
}
fn inspect_program_range_with_budget<B: CffAdmissionBudgetV2>(
    source: impl Into<SourceBytesV2>,
    source_span: Option<ProgramSpan>,
    glyph_count: u16,
    units_per_em: u16,
    max_subroutines: u32,
    metadata: Option<(&str, [i16; 4])>,
    budget: &mut B,
) -> CffProgramBudgetResultV2<CffProgramInspectionV2, B::Error> {
    let mut state = program_budget::State::new(budget);
    program_budget::finish(program_kernel(
        source.into(),
        source_span,
        glyph_count,
        units_per_em,
        max_subroutines,
        metadata,
        &mut state,
    ))
}
fn program_kernel<B: CffAdmissionBudgetV2>(
    source: SourceBytesV2,
    source_span: Option<ProgramSpan>,
    glyph_count: u16,
    units_per_em: u16,
    max_subroutines: u32,
    metadata: Option<(&str, [i16; 4])>,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<CffProgramInspectionV2, B::Error> {
    s.work(PStage::Header, Some(0), None, 8)?;
    let b: &[u8] = match source_span {
        None => &source,
        Some(span) => source
            .get(span.start..span.end)
            .ok_or_else(|| fail(K::InvalidStructure, 0))?,
    };
    s.source_length = Some(b.len());
    if glyph_count == 0 {
        return Err(fail(K::GlyphLimit, 0).into());
    }
    if b.len() < 4 || b[0] != 1 || b[1] != 0 || b[2] < 4 || !(1..=4).contains(&b[3]) {
        return Err(fail(K::InvalidStructure, 0).into());
    }
    let names = index_observed(b, usize::from(b[2]), 1, K::InvalidStructure, s)?;
    if names.objects.len() != 1 || names.objects[0].bytes(b).is_empty() {
        return Err(fail(K::InvalidStructure, names.span.start).into());
    }
    s.work(
        PStage::NameMetadata,
        Some(names.objects[0].start),
        None,
        metadata.map_or(0, |(name, _)| {
            name.len()
                .min(names.objects[0].end - names.objects[0].start) as u64
                + 1
        }),
    )?;
    if metadata.is_some_and(|(name, _)| names.objects[0].bytes(b) != name.as_bytes()) {
        return Err(fail(K::InvalidStructure, names.objects[0].start).into());
    }
    let top_index = index_observed(b, names.span.end, 1, K::InvalidStructure, s)?;
    if top_index.objects.len() != 1 {
        return Err(fail(K::InvalidStructure, top_index.span.start).into());
    }
    let strings = index_observed(b, top_index.span.end, 65000 - 391, K::InvalidStructure, s)?;
    let globals = index_observed(
        b,
        strings.span.end,
        max_subroutines as usize,
        K::SubroutineLimit,
        s,
    )?;
    let top_span = top_index.objects[0];
    let top = dict_observed(b, top_span, s)?;
    // ROS must be first in a CID Top DICT. Its absence selects name-keyed
    // grammar; CID-only operators still cannot occur in that grammar.
    let is_cid = top.first().map(|e| e.operator) == Some(0x0c1e);
    s.work(
        PStage::NumericValidation,
        Some(top_span.start),
        Some(0x0c07),
        top.len() as u64 + 8192,
    )?;
    matrix(&top, units_per_em, false, top_span.start)?;
    for e in &top {
        let v = &e.operands;
        s.work(
            PStage::NumericValidation,
            Some(e.operator_offset),
            Some(e.operator),
            1024 * (v.len() as u64 + 1),
        )?;
        let valid = match e.operator {
            0 | 1 | 2 | 3 | 4 | 0x0c00 => v.len() == 1 && valid_sid(&v[0], strings.objects.len()),
            5 => v.len() == 4 && v.iter().all(dict_operand_is_number),
            13 => matches!(v.as_slice(), [DictOperand::Integer(n)] if (0..=16777215).contains(n)),
            14 => {
                !v.is_empty()
                    && v.iter()
                        .all(|n| matches!(n, DictOperand::Integer(n) if *n >= 0))
            }
            15 | 17 => matches!(v.as_slice(), [DictOperand::Integer(n)] if *n >= 0),
            16 if !is_cid => matches!(v.as_slice(), [DictOperand::Integer(n)] if *n >= 0),
            18 if !is_cid => {
                matches!(v.as_slice(), [DictOperand::Integer(size), DictOperand::Integer(at)] if *size >= 0 && *at >= 0)
            }
            0x0c24 | 0x0c25 if is_cid => {
                matches!(v.as_slice(), [DictOperand::Integer(n)] if *n >= 0)
            }
            0x0c01 => matches!(v.as_slice(), [DictOperand::Integer(0 | 1)]),
            0x0c02..=0x0c04 => v.len() == 1 && dict_operand_is_number(&v[0]),
            0x0c1f | 0x0c20 | 0x0c23 if is_cid => v.len() == 1 && dict_operand_is_number(&v[0]),
            0x0c05 | 0x0c08 => v.len() == 1 && dict_operand_equals_integer(&v[0], 0),
            0x0c21 if is_cid => v.len() == 1 && dict_operand_equals_integer(&v[0], 0),
            0x0c06 => v.len() == 1 && dict_operand_equals_integer(&v[0], 2),
            0x0c07 => true, // Validated with profile matrix constraint above.
            0x0c1e if is_cid => {
                v.len() == 3
                    && valid_sid(&v[0], strings.objects.len())
                    && valid_sid(&v[1], strings.objects.len())
                    && matches!(v[2], DictOperand::Integer(n) if n >= 0)
            }
            0x0c22 if is_cid => {
                matches!(v.as_slice(), [DictOperand::Integer(n)] if (1..=65536).contains(n))
            }
            _ => {
                return Err(CffProgramErrorV2 {
                    operator: Some(e.operator),
                    ..fail(K::UnsupportedOperator, e.operator_offset)
                }
                .into())
            }
        };
        if !valid {
            return Err(CffProgramErrorV2 {
                operator: Some(e.operator),
                ..fail(K::InvalidStructure, e.operator_offset)
            }
            .into());
        }
    }
    if let Some((_, bbox)) = metadata {
        s.work(
            PStage::NumericValidation,
            Some(top_span.start),
            Some(5),
            top.len() as u64 + 8192,
        )?;
        let entry = top.iter().find(|e| e.operator == 5);
        if !entry.is_some_and(|e| {
            e.operands
                .iter()
                .zip(bbox)
                .all(|(n, v)| dict_operand_equals_integer(n, i32::from(v)))
        }) {
            return Err(CffProgramErrorV2 {
                operator: Some(5),
                ..fail(
                    K::InvalidStructure,
                    entry.map_or(top_span.start, |e| e.operator_offset),
                )
            }
            .into());
        }
    }
    let chars = index_observed(
        b,
        offset_observed(&top, 17, top_span.start, s)?,
        usize::from(glyph_count),
        K::GlyphLimit,
        s,
    )?;
    s.work(
        PStage::Charstrings,
        Some(chars.span.start),
        None,
        chars.objects.len() as u64 + 1,
    )?;
    if chars.objects.len() != usize::from(glyph_count)
        || chars.objects.iter().any(|s| s.start == s.end)
    {
        return Err(fail(K::InvalidStructure, chars.span.start).into());
    }
    let fds = if is_cid {
        let fds = index_observed(
            b,
            offset_observed(&top, 0x0c24, top_span.start, s)?,
            256,
            K::FontDictLimit,
            s,
        )?;
        if fds.objects.is_empty() {
            return Err(fail(K::FontDictLimit, fds.span.start).into());
        }
        Some(fds)
    } else {
        None
    };
    let fd_count = fds.as_ref().map_or(1, |fds| fds.objects.len());
    let fd_start = fds.as_ref().map_or(top_span.start, |fds| fds.span.start);
    let (fd_by_gid, fd_span) = if is_cid {
        let (map, span) = fd_select_observed(
            b,
            offset_observed(&top, 0x0c25, top_span.start, s)?,
            glyph_count,
            fd_count,
            s,
        )?;
        (map, Some(span))
    } else {
        let mut map = Vec::new();
        s.reserve(
            &mut map,
            usize::from(glyph_count),
            PSite::FdByGlyph,
            top_span.start,
        )?;
        s.work(
            PStage::ImplicitFontDict,
            Some(top_span.start),
            None,
            u64::from(glyph_count),
        )?;
        map.resize(usize::from(glyph_count), 0);
        (map, None)
    };
    let (charset, charset_span, encoding_span) = if is_cid {
        let (ids, span) = charset_observed(
            b,
            offset_observed(&top, 15, top_span.start, s)?,
            glyph_count,
            s,
        )?;
        let cid_count = integer_observed(&top, 0x0c22, top_span.start, s)?.unwrap_or(8720) as u32;
        s.work(
            PStage::CidCount,
            Some(span.start),
            None,
            ids.len() as u64 + 1,
        )?;
        if ids.iter().any(|cid| u32::from(*cid) >= cid_count) {
            return Err(fail(K::InvalidCharset, span.start).into());
        }
        (SourceCharset::Cid(ids), Some(span), None)
    } else {
        let at = integer_observed(&top, 15, top_span.start, s)?.unwrap_or(0) as usize;
        let (ids, span) = name_keyed::charset(b, at, glyph_count, strings.objects.len(), s)?;
        let at = integer_observed(&top, 16, top_span.start, s)?.unwrap_or(0) as usize;
        let encoding = name_keyed::encoding(b, at, glyph_count, &ids, s)?;
        (SourceCharset::Name(ids), span, encoding)
    };
    let mut spans = Vec::new();
    s.reserve(
        &mut spans,
        if is_cid { 5 + fd_count * 2 } else { 6 },
        PSite::StructureSpans,
        fd_start,
    )?;
    s.work(PStage::SpanSort, None, None, if is_cid { 5 } else { 4 })?;
    spans.push((
        ProgramSpan {
            start: 0,
            end: globals.span.end,
        },
        0u8,
    ));
    spans.push((chars.span, 0));
    if let Some(fds) = &fds {
        spans.push((fds.span, 0));
    }
    if let Some(span) = fd_span {
        spans.push((span, 0));
    }
    if let Some(span) = charset_span {
        spans.push((span, 0));
    }
    if let Some(span) = encoding_span {
        spans.push((span, 0));
    }
    let mut font_dicts = Vec::new();
    s.reserve(&mut font_dicts, fd_count, PSite::FontDicts, fd_start)?;
    let mut subroutine_count = globals.objects.len() as u64;
    for fd in 0..fd_count {
        let span = fds.as_ref().map_or(top_span, |fds| fds.objects[fd]);
        s.fd = Some(fd as u16);
        s.work(PStage::FontDict, Some(span.start), None, 1)?;
        let result: program_budget::Result<CffFontDictV2, B::Error> = (|| {
            let parsed;
            let entries = if is_cid {
                parsed = dict_observed(b, span, s)?;
                let entries = &parsed;
                s.work(
                    PStage::NumericValidation,
                    Some(span.start),
                    Some(0x0c07),
                    entries.len() as u64 + 8192,
                )?;
                matrix(entries, units_per_em, true, span.start)?;
                for e in entries {
                    s.work(
                        PStage::FontDict,
                        Some(e.operator_offset),
                        Some(e.operator),
                        1,
                    )?;
                    match e.operator {
                        18 => (),
                        0x0c26
                            if e.operands.len() == 1
                                && valid_sid(&e.operands[0], strings.objects.len()) => {}
                        _ => {
                            return Err(CffProgramErrorV2 {
                                operator: Some(e.operator),
                                ..fail(K::UnsupportedOperator, e.operator_offset)
                            }
                            .into())
                        }
                    }
                }
                entries
            } else {
                &top
            };
            s.work(
                PStage::DictLookup,
                Some(span.start),
                Some(18),
                entries.len() as u64 + 1,
            )?;
            let Some([DictOperand::Integer(size), DictOperand::Integer(at)]) =
                dict_entry(entries, 18)
            else {
                return Err(fail(K::InvalidStructure, span.start).into());
            };
            let at = usize::try_from(*at).map_err(|_| fail(K::InvalidStructure, span.start))?;
            let size = usize::try_from(*size).map_err(|_| fail(K::InvalidStructure, span.start))?;
            let private_span = range(b, at, size)?;
            let private = dict_observed(b, private_span, s)?;
            s.work(
                PStage::PrivateDict,
                Some(at),
                None,
                1024 * (private.iter().map(|e| e.operands.len() as u64).sum::<u64>() + 8)
                    + 64 * private.len() as u64,
            )?;
            validate_private_dict(&private).map_err(|_| fail(K::InvalidStructure, at))?;
            let default_width_x = dict_fixed(&private, 20)
                .map_err(|_| fail(K::InvalidStructure, at))?
                .unwrap_or(0);
            let nominal_width_x = dict_fixed(&private, 21)
                .map_err(|_| fail(K::InvalidStructure, at))?
                .unwrap_or(0);
            if size > 0 {
                spans.push((private_span, 1));
            }
            let local_subrs = match integer_observed(&private, 19, at, s)? {
                None => Vec::new(),
                Some(relative) => {
                    let position = at
                        .checked_add(
                            usize::try_from(relative).map_err(|_| fail(K::InvalidStructure, at))?,
                        )
                        .ok_or_else(|| fail(K::InvalidStructure, at))?;
                    if position < private_span.end {
                        return Err(fail(K::OverlappingStructures, position).into());
                    }
                    let remaining = u64::from(max_subroutines)
                        .checked_sub(subroutine_count)
                        .ok_or_else(|| {
                            limited(
                                K::SubroutineLimit,
                                position,
                                u64::from(max_subroutines),
                                subroutine_count,
                            )
                        })?;
                    let local =
                        index_observed(b, position, remaining as usize, K::SubroutineLimit, s)
                            .map_err(|e| match e {
                                program_budget::Failure::Native(mut e) => {
                                    if e.kind == K::SubroutineLimit {
                                        e.limit = Some(u64::from(max_subroutines));
                                        e.observed = e.observed.map(|n| n + subroutine_count);
                                    }
                                    program_budget::Failure::Native(e)
                                }
                                other => other,
                            })?;
                    subroutine_count += local.objects.len() as u64;
                    spans.push((local.span, 2));
                    local.objects
                }
            };
            Ok(CffFontDictV2 {
                private_span,
                local_subrs,
                default_width_x,
                nominal_width_x,
            })
        })();
        font_dicts.push(result.map_err(|e| e.fd(fd as u16))?);
    }
    s.fd = None;
    if B::OBSERVE_PROGRAM {
        program_budget::sort_spans(&mut spans, s)?;
    } else {
        spans.sort_unstable();
    }
    // Exact sharing between FD Private/Subrs is legal but still charged per FD.
    for pair in spans.windows(2) {
        s.work(PStage::SpanOverlap, None, None, 1)?;
        let (left, kind) = pair[0];
        let (right, next_kind) = pair[1];
        let permitted_share = left == right && kind != 0 && kind == next_kind;
        if left.end > right.start && !permitted_share {
            return Err(fail(K::OverlappingStructures, right.start).into());
        }
    }
    Ok(CffProgramInspectionV2 {
        program: CffProgramV2 {
            source,
            source_span,
            charstrings: chars.objects,
            global_subrs: globals.objects,
            font_dicts,
            fd_by_gid,
            charset,
        },
    })
}

#[cfg(test)]
fn fd_select(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    fd_count: usize,
) -> Result<(Vec<u8>, ProgramSpan), CffProgramErrorV2> {
    let mut budget = admission_budget::UnobservedAdmission;
    let mut state = program_budget::State::new(&mut budget);
    match program_budget::finish(fd_select_observed(b, at, glyph_count, fd_count, &mut state)) {
        Ok(native) => native,
        Err(e) => match e.error {},
    }
}
fn fd_select_observed<B: CffAdmissionBudgetV2>(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    fd_count: usize,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<(Vec<u8>, ProgramSpan), B::Error> {
    s.source_length = Some(b.len());
    s.work(PStage::FdSelectHeader, Some(at), None, 16)?;
    let count = usize::from(glyph_count);
    let format = *b.get(at).ok_or_else(|| fail(K::InvalidFdSelect, at))?;
    let mut output = Vec::new();
    s.reserve(&mut output, count, PSite::FdByGlyph, at)?;
    let span = match format {
        0 => {
            let span = range(b, at, count + 1)?;
            for (i, fd) in b[at + 1..span.end].iter().copied().enumerate() {
                s.work(PStage::FdSelectFill, Some(at + 1 + i), None, 1)?;
                if usize::from(fd) >= fd_count {
                    return Err(fail(K::InvalidFdSelect, at + 1 + i).into());
                }
                output.push(fd);
            }
            span
        }
        3 => {
            let ranges = usize::from(u16_at(b, at + 1)?);
            if ranges == 0 || ranges > count {
                return Err(fail(K::InvalidFdSelect, at + 1).into());
            }
            let span = range(b, at, 3 + ranges * 3 + 2)?;
            if u16_at(b, span.end - 2)? != glyph_count {
                return Err(fail(K::InvalidFdSelect, span.end - 2).into());
            }
            for i in 0..ranges {
                s.work(PStage::FdSelectRange, Some(at + 3 + i * 3), None, 1)?;
                let start = usize::from(u16_at(b, at + 3 + i * 3)?);
                let end = usize::from(u16_at(b, at + 6 + i * 3)?);
                let fd = b[at + 5 + i * 3];
                if start != output.len()
                    || start >= end
                    || end > count
                    || usize::from(fd) >= fd_count
                {
                    return Err(fail(K::InvalidFdSelect, at + 3 + i * 3).into());
                }
                while output.len() < end {
                    let next = (output.len() + 256).min(end);
                    s.work(
                        PStage::FdSelectFill,
                        Some(at + 3 + i * 3),
                        None,
                        (next - output.len()) as u64,
                    )?;
                    output.resize(next, fd);
                }
            }
            span
        }
        _ => return Err(fail(K::InvalidFdSelect, at).into()),
    };
    Ok((output, span))
}
#[cfg(test)]
fn charset(
    b: &[u8],
    at: usize,
    glyph_count: u16,
) -> Result<(Vec<u16>, ProgramSpan), CffProgramErrorV2> {
    let mut budget = admission_budget::UnobservedAdmission;
    let mut state = program_budget::State::new(&mut budget);
    match program_budget::finish(charset_observed(b, at, glyph_count, &mut state)) {
        Ok(native) => native,
        Err(e) => match e.error {},
    }
}
fn charset_observed<B: CffAdmissionBudgetV2>(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<(Vec<u16>, ProgramSpan), B::Error> {
    custom_charset(b, at, glyph_count, PSite::CidByGlyph, s)
}
fn custom_charset<B: CffAdmissionBudgetV2>(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    site: PSite,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<(Vec<u16>, ProgramSpan), B::Error> {
    s.source_length = Some(b.len());
    s.work(PStage::CharsetHeader, Some(at), None, 16)?;
    if at <= 2 {
        return Err(fail(K::InvalidCharset, at).into());
    } // No predefined CID charsets.
    let format = *b.get(at).ok_or_else(|| fail(K::InvalidCharset, at))?;
    if format > 2 {
        return Err(fail(K::InvalidCharset, at).into());
    }
    let mut output = Vec::new();
    s.reserve(&mut output, usize::from(glyph_count).max(1), site, at)?;
    s.work(PStage::CharsetFill, Some(at), None, 1)?;
    output.push(0);
    s.work(PStage::CharsetHeader, Some(at), None, 1024)?;
    let mut seen = [0u64; 1024];
    seen[0] = 1;
    let mut cursor = at + 1;
    while output.len() < usize::from(glyph_count) {
        s.work(PStage::CharsetRange, Some(cursor), None, 1)?;
        let first = u16_at(b, cursor)?;
        cursor += 2;
        let left = match format {
            0 => 0,
            1 => {
                let n = *b
                    .get(cursor)
                    .ok_or_else(|| fail(K::InvalidCharset, cursor))?;
                cursor += 1;
                u16::from(n)
            }
            2 => {
                let n = u16_at(b, cursor)?;
                cursor += 2;
                n
            }
            _ => unreachable!(),
        };
        let last = first
            .checked_add(left)
            .ok_or_else(|| fail(K::InvalidCharset, cursor))?;
        if output.len() + usize::from(left) + 1 > usize::from(glyph_count) {
            return Err(fail(K::InvalidCharset, cursor).into());
        }
        for cid in first..=last {
            s.work(PStage::CharsetFill, Some(cursor), None, 1)?;
            let word = &mut seen[usize::from(cid) / 64];
            let mask = 1u64 << (cid % 64);
            if *word & mask != 0 {
                return Err(fail(K::InvalidCharset, cursor).into());
            }
            *word |= mask;
            output.push(cid);
        }
    }
    Ok((
        output,
        ProgramSpan {
            start: at,
            end: cursor,
        },
    ))
}

#[cfg(test)]
#[path = "cff_v2_tests.rs"]
mod tests;

#[path = "cff_v2_evaluate.rs"]
mod evaluate;
pub use evaluate::{
    CffEvaluatedGlyphV2, CffGlyphFailureReasonV2, CffGlyphFailureV2, CffOutlineCommandV2,
    CffProgramEvaluationSessionV2,
};

#[cfg(test)]
#[path = "cff_v2_evaluate_tests.rs"]
mod evaluate_tests;

#[path = "cff_v2_vertical.rs"]
pub(super) mod vertical;
pub use vertical::{
    validate_cff_vertical_metrics_v2, CffTableFailureKindV2, CffTableFailureV2,
    CffVerticalMetricsV2,
};

#[path = "cff_v2_variations.rs"]
mod variations;
pub use variations::{
    validate_cff_variation_sequences_v2, CffVariationSequencesV2, VariationCoverage,
};

#[path = "cff_v2_cmap.rs"]
mod cmap;
pub use cmap::{
    validate_cff_cmap_v2, validate_cff_cmap_v2_with_base_cmap_budget,
    validate_cff_cmap_v2_with_cmap_budget, CffCmapFailureV2, CffCmapV2,
};

#[path = "cff_v2_admission.rs"]
mod admission;
pub use admission::{
    admit_sfnt_cff1_v2, admit_sfnt_cff1_v2_owned, admit_sfnt_cff1_v2_owned_with_admission_budget,
    admit_sfnt_cff1_v2_owned_with_base_cmap_budget, admit_sfnt_cff1_v2_owned_with_cmap_budget,
    admit_sfnt_cff1_v2_with_admission_budget, admit_sfnt_cff1_v2_with_base_cmap_budget,
    admit_sfnt_cff1_v2_with_cmap_budget, Cff1AdmissionV2, Cff1FailureKindV2, Cff1FailureV2,
};

#[path = "cff_v2_selection.rs"]
mod selection;
pub use selection::{Cff1GlyphClosureV2, Cff1SubsetSessionV2, CffSelectionFailureV2};

#[path = "cff_v2_subset.rs"]
mod subset;
pub use subset::Cff1SubsetV2;

#[path = "cff_v2_base_cmap_budget.rs"]
mod base_cmap_budget;
pub use base_cmap_budget::{
    CffBaseCmapBudgetFailureV2, CffBaseCmapBudgetResultV2, CffBaseCmapBudgetV2,
    CffBaseCmapStorageV2,
};

#[cfg(test)]
#[path = "cff_v2_base_cmap_cases.rs"]
mod base_cmap_cases;

#[path = "cff_v2_cmap_budget.rs"]
mod cmap_budget;
pub use cmap_budget::{
    CffCmapBudgetFailureV2, CffCmapBudgetResultV2, CffCmapBudgetV2, CffVariationBudgetFailureV2,
    CffVariationStorageV2,
};

#[cfg(test)]
#[path = "cff_v2_variation_budget_cases.rs"]
mod variation_budget_cases;

#[path = "cff_v2_admission_budget.rs"]
pub(super) mod admission_budget;
pub use admission_budget::{
    CffAdmissionBudgetFailureV2, CffAdmissionBudgetResultV2, CffAdmissionBudgetV2,
    CffMetadataBudgetFailureV2, CffMetadataStageV2, CffMetadataStorageV2, CffMetadataWorkPointV2,
    CffSfntBudgetFailureV2, CffSfntStageV2, CffSfntStorageV2, CffSfntWorkPointV2,
    CffTableAdmissionBudgetFailureV2, CffTableAdmissionStageV2, CffTableAdmissionWorkPointV2,
};

#[path = "cff_v2_program_budget.rs"]
mod program_budget;
pub use program_budget::{
    CffProgramBudgetFailureV2, CffProgramBudgetResultV2, CffProgramStageV2, CffProgramStorageV2,
    CffProgramWorkPointV2,
};
