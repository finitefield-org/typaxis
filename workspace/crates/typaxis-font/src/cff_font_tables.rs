//! A fixed stack index for the validated /2 SFNT table domain. Work allowances
//! cover bounded local operations; optional library walks are separate.
use super::*;
use CffTableAdmissionStageV2 as Stage;
const TAGS: [[u8; 4]; 19] = [
    *b"BASE", *b"CFF ", *b"GDEF", *b"GPOS", *b"GSUB", *b"JSTF", *b"MATH", *b"OS/2", *b"VORG",
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"kern", *b"maxp", *b"name", *b"post", *b"vhea",
    *b"vmtx",
];
// Each index query compares no more than all 19 fixed four-byte tags.
pub(super) const LOOKUP_ALLOWANCE: u64 = TAGS.len() as u64;
pub(super) fn point(
    stage: Stage,
    tag: Option<[u8; 4]>,
    offset: Option<usize>,
) -> CffTableAdmissionWorkPointV2 {
    CffTableAdmissionWorkPointV2 {
        stage,
        table_tag: tag,
        file_offset: None,
        table_offset: offset.map(|n| n as u64),
    }
}
pub(super) fn work<B: CffAdmissionBudgetV2>(
    budget: &mut B,
    p: CffTableAdmissionWorkPointV2,
    n: u64,
) -> Result<(), CffTableAdmissionBudgetFailureV2<B::Error>> {
    if B::OBSERVE_TABLES && n != 0 {
        budget
            .work_tables(n)
            .map_err(|error| CffTableAdmissionBudgetFailureV2 { error, point: p })
    } else {
        Ok(())
    }
}
pub(super) trait Lookup<'a> {
    fn get(&self, tag: &[u8; 4]) -> Option<&TableRef<'a>>;
    fn contains_key(&self, tag: &[u8; 4]) -> bool {
        self.get(tag).is_some()
    }
}
impl<'a> Lookup<'a> for BTreeMap<[u8; 4], TableRef<'a>> {
    fn get(&self, tag: &[u8; 4]) -> Option<&TableRef<'a>> {
        BTreeMap::get(self, tag)
    }
}
#[derive(Debug)]
pub(super) struct Tables<'a> {
    ordinary: Option<BTreeMap<[u8; 4], TableRef<'a>>>,
    slots: [Option<TableRef<'a>>; 19],
}
impl<'a> Lookup<'a> for Tables<'a> {
    fn get(&self, tag: &[u8; 4]) -> Option<&TableRef<'a>> {
        if let Some(map) = &self.ordinary {
            map.get(tag)
        } else {
            TAGS.binary_search(tag)
                .ok()
                .and_then(|i| self.slots[i].as_ref())
        }
    }
}
impl<'a> std::ops::Index<&[u8; 4]> for Tables<'a> {
    type Output = TableRef<'a>;
    fn index(&self, tag: &[u8; 4]) -> &Self::Output {
        self.get(tag)
            .expect("required table was established by SFNT preflight")
    }
}
type TableIndexResult<T, E> = Result<Result<T, Cff1Error>, CffTableAdmissionBudgetFailureV2<E>>;
impl<'a> Tables<'a> {
    pub(super) fn new<B: CffAdmissionBudgetV2>(
        source: &'a [u8],
        records: &[TableRecord],
        budget: &mut B,
    ) -> TableIndexResult<Self, B::Error> {
        if !B::OBSERVE_TABLES {
            return Ok(table_map(source, records).map(|map| Self {
                ordinary: Some(map),
                slots: [None; 19],
            }));
        }
        work(budget, point(Stage::DirectoryIndex, None, None), 19)?;
        let mut slots = [None; 19];
        for (index, record) in records.iter().enumerate() {
            let mut p = point(Stage::DirectoryIndex, Some(record.tag), None);
            p.file_offset = Some((12 + index * 16) as u64);
            // 19 tag comparisons plus fixed range/duplicate checks and one slot write.
            work(budget, p, 32)?;
            let Some(end) = record.offset.checked_add(record.length) else {
                return Ok(Err(Cff1Error::InvalidSfnt));
            };
            let Some(bytes) = source.get(record.offset..end) else {
                return Ok(Err(Cff1Error::InvalidSfnt));
            };
            // FFTM is metadata validated by the /2 SFNT preflight. It has no
            // decoder or lookup slot. The existing 32-unit record allowance
            // covers this comparison and preserves other fonts' accounting.
            if record.tag == *b"FFTM" {
                continue;
            }
            let Ok(slot) = TAGS.binary_search(&record.tag) else {
                return Ok(Err(Cff1Error::InvalidSfnt));
            };
            if slots[slot].is_some() {
                return Ok(Err(Cff1Error::InvalidSfnt));
            }
            slots[slot] = Some(TableRef { bytes });
        }
        // At most three directory walks on a terminal table/cmap/metadata error.
        work(
            budget,
            point(Stage::ErrorContext, None, None),
            3 * records.len() as u64,
        )?;
        Ok(Ok(Self {
            ordinary: None,
            slots,
        }))
    }
}
/// All five fixed decoders inspect at most 54 bytes and a bounded number of
/// fields. A 64-unit allowance is paid before invoking any native decoder.
pub(super) fn fixed<B: CffAdmissionBudgetV2, T>(
    budget: &mut B,
    tag: [u8; 4],
    decode: impl FnOnce() -> T,
) -> Result<T, CffTableAdmissionBudgetFailureV2<B::Error>> {
    work(budget, point(Stage::FixedDecoder, Some(tag), None), 64)?;
    Ok(decode())
}
#[cfg(test)]
#[path = "cff_font_tables_tests.rs"]
pub(crate) mod tests;
