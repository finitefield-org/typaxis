//! Allocation/work bounds for the existing CFF table encoders. They depend
//! on selected programs and actual source tables, never the font-byte maximum.
use super::*;
pub(super) type Charge<'a> = &'a mut dyn FnMut(usize, usize, usize) -> Result<(), Cff1Error>;
pub(super) fn add(a: usize, b: usize) -> Result<usize, Cff1Error> {
    a.checked_add(b).ok_or(Cff1Error::SubsetByteLimit)
}
pub(super) fn mul(a: usize, b: usize) -> Result<usize, Cff1Error> {
    a.checked_mul(b).ok_or(Cff1Error::SubsetByteLimit)
}
pub(super) fn cid_tables(n: usize, data: usize, charge: Charge<'_>) -> Result<(), Cff1Error> {
    // INDEX: 3 header bytes, at most four per offset, then actual data.
    let index = add(add(3, mul(add(n, 1)?, 4)?)?, data)?;
    // Charset 2n, FDSelect n, and <=512 fixed DICT/INDEX/header bytes.
    let cff = add(add(512, mul(n, 3)?)?, index)?;
    // 32 convergence rounds, each with small DICT/INDEX temporaries below
    // 1024 bytes, plus the final growable Vec's <2*length capacity.
    let fixed = 32 * 1024;
    let bytes = add(add(add(index, mul(n, 3)?)?, fixed)?, mul(cff, 2)?)?;
    charge(
        add(mul(n, 3)?, 161)?,
        bytes,
        add(mul(bytes, 2)?, mul(n, 8)?)?,
    )
}
pub(super) fn copied_tables(
    admission: &Cff1AdmissionV2,
    n: usize,
    charge: Charge<'_>,
) -> Result<(), Cff1Error> {
    let mut bytes = 6usize; // maxp
    for tag in [b"head", b"hhea", b"OS/2", b"post"] {
        bytes = add(bytes, admission.table_bytes(tag)?.len())?;
    }
    bytes = add(bytes, mul(n, 4)?)?; // hmtx
    let (family, style) = admission.family_names();
    // Original UTF-8 length bounds UTF-16 storage; include both temporary
    // encoded strings and the complete name table with its 42-byte header.
    let names = add(42, mul(add(add(family.len(), style.len())?, 14)?, 4)?)?;
    bytes = add(bytes, names)?;
    bytes = add(bytes, 9 * std::mem::size_of::<RewriteTable>())?;
    charge(add(n, 12)?, bytes, add(mul(bytes, 2)?, mul(n, 16)?)?)
}
