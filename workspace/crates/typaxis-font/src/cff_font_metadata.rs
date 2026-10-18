//! Requested hmtx/name storage and bounded metadata work. Table maps, other
//! tables, source/program copies and CFF program inspection remain separate.
use super::*;
use typaxis_core::Sha256;
use CffMetadataStageV2 as Stage;
use CffMetadataStorageV2 as Site;

type HorizontalMetrics = (Vec<u16>, Vec<i16>);
type AdmissionHashes = ([u8; 32], [u8; 32]);

#[derive(Debug)]
pub(super) enum Failure<E> {
    Native(Cff1Error),
    Allocation,
    Budget(CffMetadataBudgetFailureV2<E>),
}
impl<E> From<Cff1Error> for Failure<E> {
    fn from(e: Cff1Error) -> Self {
        Self::Native(e)
    }
}
fn point(stage: Stage, tag: Option<[u8; 4]>, offset: Option<usize>) -> CffMetadataWorkPointV2 {
    CffMetadataWorkPointV2 {
        stage,
        table_tag: tag,
        file_offset: None,
        table_offset: offset.map(|n| n as u64),
    }
}
fn work<B: CffAdmissionBudgetV2>(
    budget: &mut B,
    p: CffMetadataWorkPointV2,
    n: u64,
) -> Result<(), Failure<B::Error>> {
    budget
        .work_metadata(n)
        .map_err(|error| Failure::Budget(CffMetadataBudgetFailureV2 { error, point: p }))
}
fn reserve<B: CffAdmissionBudgetV2>(
    budget: &mut B,
    p: CffMetadataWorkPointV2,
    site: Site,
    rows: usize,
    bytes: usize,
) -> Result<(), Failure<B::Error>> {
    budget
        .reserve_metadata(site, rows, bytes)
        .map_err(|error| Failure::Budget(CffMetadataBudgetFailureV2 { error, point: p }))
}
trait Allocator {
    fn vector<T>(&mut self, out: &mut Vec<T>, count: usize) -> Result<(), ()>;
    fn string(&mut self, out: &mut String, bytes: usize) -> Result<(), ()>;
}
struct Native;
impl Allocator for Native {
    fn vector<T>(&mut self, out: &mut Vec<T>, count: usize) -> Result<(), ()> {
        out.try_reserve_exact(count).map_err(|_| ())
    }
    fn string(&mut self, out: &mut String, bytes: usize) -> Result<(), ()> {
        out.try_reserve_exact(bytes).map_err(|_| ())
    }
}
pub(super) fn hmtx<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyphs: u16,
    metrics: u16,
    budget: &mut B,
) -> Result<HorizontalMetrics, Failure<B::Error>> {
    hmtx_alloc(bytes, glyphs, metrics, budget, &mut Native)
}
fn hmtx_alloc<B: CffAdmissionBudgetV2, A: Allocator>(
    bytes: &[u8],
    glyphs: u16,
    metrics: u16,
    budget: &mut B,
    alloc: &mut A,
) -> Result<HorizontalMetrics, Failure<B::Error>> {
    let p = point(Stage::HorizontalHeader, Some(*b"hmtx"), Some(0));
    work(budget, p, 1)?;
    if metrics > glyphs {
        return Err(Cff1Error::InvalidHmtx.into());
    }
    let count = usize::from(glyphs);
    let expected = usize::from(metrics) * 4 + usize::from(glyphs - metrics) * 2;
    if bytes.len() != expected {
        return Err(Cff1Error::InvalidHmtx.into());
    }
    let mut advances = Vec::new();
    let mut bearings = Vec::new();
    reserve(
        budget,
        p,
        Site::HorizontalAdvances,
        count,
        count * std::mem::size_of::<u16>(),
    )?;
    alloc
        .vector(&mut advances, count)
        .map_err(|_| Failure::Allocation)?;
    reserve(
        budget,
        p,
        Site::HorizontalBearings,
        count,
        count * std::mem::size_of::<i16>(),
    )?;
    alloc
        .vector(&mut bearings, count)
        .map_err(|_| Failure::Allocation)?;
    let mut last = None;
    for gid in 0..glyphs {
        let offset = if gid < metrics {
            usize::from(gid) * 4
        } else {
            usize::from(metrics) * 4 + usize::from(gid - metrics) * 2
        };
        work(
            budget,
            point(Stage::HorizontalMetrics, Some(*b"hmtx"), Some(offset)),
            1,
        )?;
        if gid < metrics {
            let advance = read_u16(bytes, offset, Cff1Error::InvalidHmtx)?;
            advances.push(advance);
            bearings.push(read_i16(bytes, offset + 2, Cff1Error::InvalidHmtx)?);
            last = Some(advance);
        } else {
            advances.push(last.ok_or(Cff1Error::InvalidHmtx)?);
            bearings.push(read_i16(bytes, offset, Cff1Error::InvalidHmtx)?);
        }
    }
    Ok((advances, bearings))
}
#[derive(Clone, Copy)]
struct NameView<'a> {
    rank: (u8, u16, u16),
    bytes: &'a [u8],
    offset: usize,
    utf8_len: usize,
}
// One request precedes one decode attempt (at most two UTF-16 code units).
fn scan<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    start: usize,
    stage: Stage,
    budget: &mut B,
    mut emit: impl FnMut(char),
) -> Result<usize, Failure<B::Error>> {
    if bytes.len() % 2 != 0 {
        return Err(Cff1Error::InvalidName.into());
    }
    let units = bytes
        .chunks_exact(2)
        .map(|p| u16::from_be_bytes([p[0], p[1]]));
    let mut decoder = char::decode_utf16(units);
    let mut at = 0;
    let mut utf8_len = 0;
    while at < bytes.len() {
        work(budget, point(stage, Some(*b"name"), Some(start + at)), 1)?;
        let scalar = decoder
            .next()
            .ok_or(Cff1Error::InvalidName)?
            .map_err(|_| Cff1Error::InvalidName)?;
        if u32::from(scalar) > 0xffff || scalar.is_control() {
            return Err(Cff1Error::InvalidName.into());
        }
        at += scalar.len_utf16() * 2;
        utf8_len += scalar.len_utf8();
        emit(scalar);
    }
    if utf8_len == 0 {
        return Err(Cff1Error::InvalidName.into());
    }
    Ok(utf8_len)
}
pub(super) fn names<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    budget: &mut B,
) -> Result<NameFacts, Failure<B::Error>> {
    names_alloc(bytes, budget, &mut Native)
}
fn names_alloc<B: CffAdmissionBudgetV2, A: Allocator>(
    bytes: &[u8],
    budget: &mut B,
    alloc: &mut A,
) -> Result<NameFacts, Failure<B::Error>> {
    work(budget, point(Stage::NameHeader, Some(*b"name"), Some(0)), 1)?;
    if read_u16(bytes, 0, Cff1Error::InvalidName)? != 0 {
        return Err(Cff1Error::InvalidName.into());
    }
    let count = usize::from(read_u16(bytes, 2, Cff1Error::InvalidName)?);
    let start = usize::from(read_u16(bytes, 4, Cff1Error::InvalidName)?);
    if count == 0 || start < 6 + count * 12 || start > bytes.len() {
        return Err(Cff1Error::InvalidName.into());
    }
    let mut best: [Option<NameView<'_>>; 3] = [None; 3];
    for index in 0..count {
        let record = 6 + index * 12;
        work(
            budget,
            point(Stage::NameRecord, Some(*b"name"), Some(record)),
            1,
        )?;
        let platform = read_u16(bytes, record, Cff1Error::InvalidName)?;
        let encoding = read_u16(bytes, record + 2, Cff1Error::InvalidName)?;
        let language = read_u16(bytes, record + 4, Cff1Error::InvalidName)?;
        let id = read_u16(bytes, record + 6, Cff1Error::InvalidName)?;
        let len = usize::from(read_u16(bytes, record + 8, Cff1Error::InvalidName)?);
        let offset = start + usize::from(read_u16(bytes, record + 10, Cff1Error::InvalidName)?);
        // Even an ignored record must refer to an in-range payload.
        let payload = bytes
            .get(offset..offset + len)
            .ok_or(Cff1Error::InvalidName)?;
        let slot = match id {
            1 => 0,
            2 => 1,
            6 => 2,
            _ => continue,
        };
        let rank = match (platform, encoding, language) {
            (3, 10, 0x0409) => (0, encoding, language),
            (3, 1, 0x0409) => (1, encoding, language),
            (0, _, _) => (2, encoding, language),
            (3, 10 | 1, _) => (3, encoding, language),
            _ => continue,
        };
        // Validate every eligible candidate, including lower-ranked aliases.
        let utf8_len = scan(payload, offset, Stage::NameValidation, budget, |_| {})?;
        work(
            budget,
            point(Stage::NameSelection, Some(*b"name"), Some(record)),
            1,
        )?;
        if best[slot].map_or(true, |old| rank < old.rank) {
            best[slot] = Some(NameView {
                rank,
                bytes: payload,
                offset,
                utf8_len,
            });
        }
    }
    let mut copy = |slot: usize, site| -> Result<String, Failure<B::Error>> {
        let view = best[slot].ok_or(Cff1Error::InvalidName)?;
        let p = point(Stage::NameCopy, Some(*b"name"), Some(view.offset));
        reserve(budget, p, site, 0, view.utf8_len)?;
        let mut out = String::new();
        alloc
            .string(&mut out, view.utf8_len)
            .map_err(|_| Failure::Allocation)?;
        let copied = scan(view.bytes, view.offset, Stage::NameCopy, budget, |c| {
            out.push(c)
        })?;
        debug_assert_eq!(copied, view.utf8_len);
        debug_assert_eq!(out.len(), view.utf8_len);
        Ok(out)
    };
    let family = copy(0, Site::FamilyName)?;
    let subfamily = copy(1, Site::SubfamilyName)?;
    let postscript_name = copy(2, Site::PostscriptName)?;
    let view = best[2].unwrap();
    let p = point(
        Stage::PostscriptValidation,
        Some(*b"name"),
        Some(view.offset),
    );
    work(budget, p, 1)?;
    if postscript_name.len() > 127 {
        return Err(Cff1Error::InvalidName.into());
    }
    for (index, byte) in postscript_name.bytes().enumerate() {
        // A successful PS byte is ASCII, so its UTF-16 position is index * 2.
        work(
            budget,
            point(
                Stage::PostscriptValidation,
                Some(*b"name"),
                Some(view.offset + index * 2),
            ),
            1,
        )?;
        if !(33..=126).contains(&byte) || b"()<>[]{}/%".contains(&byte) {
            return Err(Cff1Error::InvalidName.into());
        }
    }
    if postscript_name.len() > 7 {
        let mut prefix = true;
        for (index, byte) in postscript_name.bytes().take(6).enumerate() {
            work(
                budget,
                point(
                    Stage::PostscriptValidation,
                    Some(*b"name"),
                    Some(view.offset + index * 2),
                ),
                1,
            )?;
            if !byte.is_ascii_uppercase() {
                prefix = false;
                break;
            }
        }
        if prefix {
            work(
                budget,
                point(
                    Stage::PostscriptValidation,
                    Some(*b"name"),
                    Some(view.offset + 12),
                ),
                1,
            )?;
            if postscript_name.as_bytes()[6] == b'+' {
                return Err(Cff1Error::InvalidName.into());
            }
        }
    }
    Ok(NameFacts {
        family,
        subfamily,
        postscript_name,
    })
}
fn digest<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    stage: Stage,
    budget: &mut B,
) -> Result<[u8; 32], Failure<B::Error>> {
    let mut p = point(stage, None, None);
    if stage == Stage::SourceDigest {
        p.file_offset = Some(0);
    }
    work(budget, p, 1)?;
    let mut state = Sha256::new();
    for (index, chunk) in bytes.chunks(256).enumerate() {
        if stage == Stage::SourceDigest {
            p.file_offset = Some((index * 256) as u64);
        }
        work(budget, p, chunk.len() as u64 + 1)?;
        state.update(chunk);
    }
    if stage == Stage::SourceDigest {
        p.file_offset = Some(bytes.len() as u64);
    }
    work(budget, p, 128)?;
    Ok(state.finish())
}
const PREFIX: &[u8] =
    b"{\"algorithm\":\"typaxis.sfnt-cff1-admission/2\",\"face_index\":0,\"limits_fingerprint\":";
const MIDDLE: &[u8] =
    b",\"resource_profile\":\"typaxis.resource-profile/sfnt-cff1/2\",\"source_sha256\":";
const IDENTITY_LEN: usize = PREFIX.len() + 66 + MIDDLE.len() + 66 + 1;
pub(super) fn hashes<B: CffAdmissionBudgetV2>(
    source: &[u8],
    limits: [u8; 32],
    budget: &mut B,
) -> Result<AdmissionHashes, Failure<B::Error>> {
    let source_hash = digest(source, Stage::SourceDigest, budget)?;
    let mut identity = [0u8; IDENTITY_LEN];
    let mut cursor = 0;
    let p = point(Stage::IdentityAssembly, None, None);
    for (literal, hash) in [(PREFIX, limits), (MIDDLE, source_hash)] {
        work(budget, p, literal.len() as u64)?;
        identity[cursor..cursor + literal.len()].copy_from_slice(literal);
        cursor += literal.len();
        work(budget, p, 66)?;
        identity[cursor] = b'"';
        cursor += 1;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in hash {
            identity[cursor] = HEX[usize::from(byte >> 4)];
            identity[cursor + 1] = HEX[usize::from(byte & 15)];
            cursor += 2;
        }
        identity[cursor] = b'"';
        cursor += 1;
    }
    work(budget, p, 1)?;
    identity[cursor] = b'}';
    debug_assert_eq!(cursor + 1, IDENTITY_LEN);
    let identity_hash = digest(&identity, Stage::IdentityDigest, budget)?;
    Ok((source_hash, identity_hash))
}

#[cfg(test)]
#[path = "cff_font_metadata_tests.rs"]
mod tests;
