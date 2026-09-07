//! CFF /2 program structure. This is not sfnt admission or PDF authorization.
//! Byte ranges retain the original CFF table once; no per-program byte copies.
use super::*;
use std::sync::Arc;

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
fn reserve<T>(v: &mut Vec<T>, n: usize, offset: usize) -> Result<(), CffProgramErrorV2> {
    v.try_reserve_exact(n)
        .map_err(|_| fail(K::AllocationFailure, offset))
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
fn index(b: &[u8], at: usize, maximum: usize, axis: K) -> Result<ProgramIndex, CffProgramErrorV2> {
    let count = usize::from(u16_at(b, at)?);
    if count > maximum {
        return Err(limited(axis, at, maximum as u64, count as u64));
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
        return Err(fail(K::InvalidStructure, at + 2));
    }
    let offsets = range(b, at + 3, (count + 1) * size)?;
    let mut objects = Vec::new();
    reserve(&mut objects, count, at)?;
    let mut previous = 0usize;
    for i in 0..=count {
        let offset = at + 3 + i * size;
        let value = b[offset..offset + size]
            .iter()
            .try_fold(0usize, |n, byte| {
                n.checked_mul(256)
                    .and_then(|n| n.checked_add(usize::from(*byte)))
            })
            .ok_or_else(|| fail(K::InvalidStructure, offset))?;
        if value == 0 || (i == 0 && value != 1) || value < previous {
            return Err(fail(K::InvalidStructure, offset));
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
fn dict(b: &[u8], span: ProgramSpan) -> Result<Vec<DictEntry>, CffProgramErrorV2> {
    let bytes = span.bytes(b);
    let mut entries = Vec::new();
    let mut operands = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let value = bytes[cursor];
        if value >= 28 {
            if operands.len() == 48 {
                return Err(limited(K::OperandLimit, span.start + cursor, 48, 49));
            }
            let (operand, consumed) = parse_dict_operand(bytes, cursor)
                .map_err(|_| fail(K::InvalidStructure, span.start + cursor))?;
            reserve(&mut operands, 1, span.start + cursor)?;
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
                return Err(fail(K::InvalidStructure, span.start + cursor));
            };
            if entries.iter().any(|e: &DictEntry| e.operator == operator) {
                return Err(fail(K::InvalidStructure, span.start + cursor));
            }
            reserve(&mut entries, 1, span.start + cursor)?;
            entries.push(DictEntry {
                operator_offset: span.start + cursor,
                operator,
                operands: std::mem::take(&mut operands),
            });
            cursor += consumed;
        }
    }
    if !operands.is_empty() {
        return Err(fail(K::InvalidStructure, span.end));
    }
    Ok(entries)
}
fn integer(entries: &[DictEntry], op: u16, at: usize) -> Result<Option<i32>, CffProgramErrorV2> {
    one_dict_integer(entries, op).map_err(|_| fail(K::InvalidStructure, at))
}
fn offset(entries: &[DictEntry], op: u16, at: usize) -> Result<usize, CffProgramErrorV2> {
    usize::try_from(integer(entries, op, at)?.ok_or_else(|| fail(K::InvalidStructure, at))?)
        .map_err(|_| fail(K::InvalidStructure, at))
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
    source: Arc<[u8]>,
    charstrings: Vec<ProgramSpan>,
    global_subrs: Vec<ProgramSpan>,
    font_dicts: Vec<CffFontDictV2>,
    fd_by_gid: Vec<u8>,
    cid_by_gid: Vec<u16>,
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
        self.program.cid_by_gid.get(gid).copied()
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
            .map(|s| s.bytes(&self.program.source))
    }
    pub fn private_dict_bytes(&self, fd: usize) -> Option<&[u8]> {
        self.program
            .font_dicts
            .get(fd)
            .map(|d| d.private_span.bytes(&self.program.source))
    }
}

/// Parse a CID-keyed CFF1 table for the version-2 implementation. Source is the
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
    let b: &[u8] = &source;
    if glyph_count == 0 {
        return Err(fail(K::GlyphLimit, 0));
    }
    if b.len() < 4 || b[0] != 1 || b[1] != 0 || b[2] < 4 || !(1..=4).contains(&b[3]) {
        return Err(fail(K::InvalidStructure, 0));
    }
    let names = index(b, usize::from(b[2]), 1, K::InvalidStructure)?;
    if names.objects.len() != 1 || names.objects[0].bytes(b).is_empty() {
        return Err(fail(K::InvalidStructure, names.span.start));
    }
    if metadata.is_some_and(|(name, _)| names.objects[0].bytes(b) != name.as_bytes()) {
        return Err(fail(K::InvalidStructure, names.objects[0].start));
    }
    let top_index = index(b, names.span.end, 1, K::InvalidStructure)?;
    if top_index.objects.len() != 1 {
        return Err(fail(K::InvalidStructure, top_index.span.start));
    }
    let strings = index(b, top_index.span.end, 65000 - 391, K::InvalidStructure)?;
    let globals = index(
        b,
        strings.span.end,
        max_subroutines as usize,
        K::SubroutineLimit,
    )?;
    let top_span = top_index.objects[0];
    let top = dict(b, top_span)?;
    if top.first().map(|e| e.operator) != Some(0x0c1e) {
        return Err(fail(K::ExpectedCidFont, top_span.start));
    }
    matrix(&top, units_per_em, false, top_span.start)?;
    for e in &top {
        let v = &e.operands;
        let valid = match e.operator {
            0 | 1 | 2 | 3 | 4 | 0x0c00 => v.len() == 1 && valid_sid(&v[0], strings.objects.len()),
            5 => v.len() == 4 && v.iter().all(dict_operand_is_number),
            13 => matches!(v.as_slice(), [DictOperand::Integer(n)] if (0..=16777215).contains(n)),
            14 => {
                !v.is_empty()
                    && v.iter()
                        .all(|n| matches!(n, DictOperand::Integer(n) if *n >= 0))
            }
            15 | 17 | 0x0c24 | 0x0c25 => {
                matches!(v.as_slice(), [DictOperand::Integer(n)] if *n >= 0)
            }
            0x0c01 => matches!(v.as_slice(), [DictOperand::Integer(0 | 1)]),
            0x0c02 | 0x0c03 | 0x0c04 | 0x0c1f | 0x0c20 | 0x0c23 => {
                v.len() == 1 && dict_operand_is_number(&v[0])
            }
            0x0c05 | 0x0c08 | 0x0c21 => v.len() == 1 && dict_operand_equals_integer(&v[0], 0),
            0x0c06 => v.len() == 1 && dict_operand_equals_integer(&v[0], 2),
            0x0c07 => true, // Validated with profile matrix constraint above.
            0x0c1e => {
                v.len() == 3
                    && valid_sid(&v[0], strings.objects.len())
                    && valid_sid(&v[1], strings.objects.len())
                    && matches!(v[2], DictOperand::Integer(n) if n >= 0)
            }
            0x0c22 => matches!(v.as_slice(), [DictOperand::Integer(n)] if (1..=65536).contains(n)),
            _ => {
                return Err(CffProgramErrorV2 {
                    operator: Some(e.operator),
                    ..fail(K::UnsupportedOperator, e.operator_offset)
                })
            }
        };
        if !valid {
            return Err(CffProgramErrorV2 {
                operator: Some(e.operator),
                ..fail(K::InvalidStructure, e.operator_offset)
            });
        }
    }
    if let Some((_, bbox)) = metadata {
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
            });
        }
    }
    let chars = index(
        b,
        offset(&top, 17, top_span.start)?,
        usize::from(glyph_count),
        K::GlyphLimit,
    )?;
    if chars.objects.len() != usize::from(glyph_count)
        || chars.objects.iter().any(|s| s.start == s.end)
    {
        return Err(fail(K::InvalidStructure, chars.span.start));
    }
    let fds = index(
        b,
        offset(&top, 0x0c24, top_span.start)?,
        256,
        K::FontDictLimit,
    )?;
    if fds.objects.is_empty() {
        return Err(fail(K::FontDictLimit, fds.span.start));
    }
    let (fd_by_gid, fd_span) = fd_select(
        b,
        offset(&top, 0x0c25, top_span.start)?,
        glyph_count,
        fds.objects.len(),
    )?;
    let (cid_by_gid, charset_span) = charset(b, offset(&top, 15, top_span.start)?, glyph_count)?;
    let cid_count = integer(&top, 0x0c22, top_span.start)?.unwrap_or(8720) as u32;
    if cid_by_gid.iter().any(|cid| u32::from(*cid) >= cid_count) {
        return Err(fail(K::InvalidCharset, charset_span.start));
    }
    let mut spans = vec![
        (
            ProgramSpan {
                start: 0,
                end: globals.span.end,
            },
            0u8,
        ),
        (chars.span, 0),
        (fds.span, 0),
        (fd_span, 0),
        (charset_span, 0),
    ];
    reserve(&mut spans, fds.objects.len() * 2, fds.span.start)?;
    let mut font_dicts = Vec::new();
    reserve(&mut font_dicts, fds.objects.len(), fds.span.start)?;
    let mut subroutine_count = globals.objects.len() as u64;
    for (fd, span) in fds.objects.iter().copied().enumerate() {
        let result = (|| {
            let entries = dict(b, span)?;
            matrix(&entries, units_per_em, true, span.start)?;
            for e in &entries {
                match e.operator {
                    18 => (),
                    0x0c26
                        if e.operands.len() == 1
                            && valid_sid(&e.operands[0], strings.objects.len()) =>
                    {
                        ()
                    }
                    _ => {
                        return Err(CffProgramErrorV2 {
                            operator: Some(e.operator),
                            ..fail(K::UnsupportedOperator, e.operator_offset)
                        })
                    }
                }
            }
            let Some([DictOperand::Integer(size), DictOperand::Integer(at)]) =
                dict_entry(&entries, 18)
            else {
                return Err(fail(K::InvalidStructure, span.start));
            };
            let at = usize::try_from(*at).map_err(|_| fail(K::InvalidStructure, span.start))?;
            let size = usize::try_from(*size).map_err(|_| fail(K::InvalidStructure, span.start))?;
            let private_span = range(b, at, size)?;
            let private = dict(b, private_span)?;
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
            let local_subrs = match integer(&private, 19, at)? {
                None => Vec::new(),
                Some(relative) => {
                    let position = at
                        .checked_add(
                            usize::try_from(relative).map_err(|_| fail(K::InvalidStructure, at))?,
                        )
                        .ok_or_else(|| fail(K::InvalidStructure, at))?;
                    if position < private_span.end {
                        return Err(fail(K::OverlappingStructures, position));
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
                    let local = index(b, position, remaining as usize, K::SubroutineLimit)
                        .map_err(|mut e| {
                            if e.kind == K::SubroutineLimit {
                                e.limit = Some(u64::from(max_subroutines));
                                e.observed = e.observed.map(|n| n + subroutine_count);
                            }
                            e
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
        font_dicts.push(result.map_err(|mut e: CffProgramErrorV2| {
            e.fd = Some(fd as u16);
            e
        })?);
    }
    spans.sort_unstable();
    // Exact sharing between FD Private/Subrs is legal but still charged per FD.
    for pair in spans.windows(2) {
        let (left, kind) = pair[0];
        let (right, next_kind) = pair[1];
        let permitted_share = left == right && kind != 0 && kind == next_kind;
        if left.end > right.start && !permitted_share {
            return Err(fail(K::OverlappingStructures, right.start));
        }
    }
    Ok(CffProgramInspectionV2 {
        program: CffProgramV2 {
            source,
            charstrings: chars.objects,
            global_subrs: globals.objects,
            font_dicts,
            fd_by_gid,
            cid_by_gid,
        },
    })
}

fn fd_select(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    fd_count: usize,
) -> Result<(Vec<u8>, ProgramSpan), CffProgramErrorV2> {
    let count = usize::from(glyph_count);
    let format = *b.get(at).ok_or_else(|| fail(K::InvalidFdSelect, at))?;
    let mut output = Vec::new();
    reserve(&mut output, count, at)?;
    let span = match format {
        0 => {
            let span = range(b, at, count + 1)?;
            for (i, fd) in b[at + 1..span.end].iter().copied().enumerate() {
                if usize::from(fd) >= fd_count {
                    return Err(fail(K::InvalidFdSelect, at + 1 + i));
                }
                output.push(fd);
            }
            span
        }
        3 => {
            let ranges = usize::from(u16_at(b, at + 1)?);
            if ranges == 0 || ranges > count {
                return Err(fail(K::InvalidFdSelect, at + 1));
            }
            let span = range(b, at, 3 + ranges * 3 + 2)?;
            if u16_at(b, span.end - 2)? != glyph_count {
                return Err(fail(K::InvalidFdSelect, span.end - 2));
            }
            for i in 0..ranges {
                let start = usize::from(u16_at(b, at + 3 + i * 3)?);
                let end = usize::from(u16_at(b, at + 6 + i * 3)?);
                let fd = b[at + 5 + i * 3];
                if start != output.len()
                    || start >= end
                    || end > count
                    || usize::from(fd) >= fd_count
                {
                    return Err(fail(K::InvalidFdSelect, at + 3 + i * 3));
                }
                output.resize(end, fd);
            }
            span
        }
        _ => return Err(fail(K::InvalidFdSelect, at)),
    };
    Ok((output, span))
}
fn charset(
    b: &[u8],
    at: usize,
    glyph_count: u16,
) -> Result<(Vec<u16>, ProgramSpan), CffProgramErrorV2> {
    if at <= 2 {
        return Err(fail(K::InvalidCharset, at));
    } // No predefined CID charsets.
    let format = *b.get(at).ok_or_else(|| fail(K::InvalidCharset, at))?;
    if format > 2 {
        return Err(fail(K::InvalidCharset, at));
    }
    let mut output = Vec::new();
    reserve(&mut output, usize::from(glyph_count), at)?;
    output.push(0);
    let mut seen = [0u64; 1024];
    seen[0] = 1;
    let mut cursor = at + 1;
    while output.len() < usize::from(glyph_count) {
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
            return Err(fail(K::InvalidCharset, cursor));
        }
        for cid in first..=last {
            let word = &mut seen[usize::from(cid) / 64];
            let mask = 1u64 << (cid % 64);
            if *word & mask != 0 {
                return Err(fail(K::InvalidCharset, cursor));
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
mod vertical;
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
pub use cmap::{validate_cff_cmap_v2, CffCmapFailureV2, CffCmapV2};

#[path = "cff_v2_admission.rs"]
mod admission;
pub use admission::{admit_sfnt_cff1_v2, Cff1AdmissionV2, Cff1FailureKindV2, Cff1FailureV2};

#[path = "cff_v2_selection.rs"]
mod selection;
pub use selection::{Cff1GlyphClosureV2, Cff1SubsetSessionV2, CffSelectionFailureV2};
