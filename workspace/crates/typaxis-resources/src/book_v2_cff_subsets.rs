//! Original CFF /2 subset programs, tied to actual closed book font uses.
use super::*;
pub const BOOK_V2_CFF_SUBSET_ALGORITHM: &str = "typaxis.book-2-cff-subset/1";
#[derive(Debug)]
pub enum BookV2CffSubsetError {
    Budget(BookV2FontSelectionError),
    WrongFontKind,
    InvalidFontIndex,
    Evaluation(BookV2CffProgramError),
    Font(typaxis_font::Cff1Error),
}
impl From<E> for BookV2CffSubsetError {
    fn from(e: E) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2CffSubsetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 CFF subset: {self:?}")
    }
}
impl std::error::Error for BookV2CffSubsetError {}
pub struct BookV2CffSubset<'x, 'c, 'a> {
    source: &'x BookV2ClosedFont<'c, 'a>,
    program: typaxis_font::Cff1SubsetV2,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'x, 'c, 'a> BookV2CffSubset<'x, 'c, 'a> {
    pub fn source(&self) -> &'x BookV2ClosedFont<'c, 'a> {
        self.source
    }
    pub fn program(&self) -> &typaxis_font::Cff1SubsetV2 {
        &self.program
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn write_cff_font<'x, 'c>(
        &mut self,
        closures: &'x BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        index: usize,
    ) -> Result<BookV2CffSubset<'x, 'c, 'a>, BookV2CffSubsetError> {
        if !std::ptr::eq(self.display, closures.selection().display()) {
            return Err(E::Identity.into());
        }
        let source = closures
            .fonts()
            .get(index)
            .ok_or(BookV2CffSubsetError::InvalidFontIndex)?;
        let Closure::Cff(closed) = &source.closure else {
            return Err(BookV2CffSubsetError::WrongFontKind);
        };
        let AdmittedProductionFontV3::Cff1V2(font) = source.source().instance().font() else {
            return Err(E::Identity.into());
        };
        // Evaluate all closed faces in canonical order, not just the font
        // requested first. This also inherits all prior closure charges.
        self.prepare_cff_programs(closures).map_err(|e| match e {
            BookV2CffProgramError::Budget(e) => BookV2CffSubsetError::Budget(e),
            e => BookV2CffSubsetError::Evaluation(e),
        })?;
        let n = closed.source_gids().len();
        let bytes = n
            .checked_mul(2)
            .and_then(|n| n.checked_add(closed.canonical_jcs().len()))
            .ok_or(E::Spool)?;
        self.closure_charge(
            n.checked_add(3).ok_or(E::Records)?,
            bytes,
            n.checked_add(closed.canonical_jcs().len().div_ceil(64))
                .and_then(|n| n.checked_add(1))
                .ok_or(E::Work)?,
        )?;
        let closed = closed.clone();
        let session = self.cff_session.take().ok_or(E::Identity)?;
        let mut failure = None;
        let result = session.write_prepared_subset_with_charge(
            font.admission(),
            closed,
            &mut |records, bytes, work| {
                self.closure_charge(records, bytes, work).map_err(|e| {
                    failure = Some(e);
                    typaxis_font::Cff1Error::SubsetByteLimit
                })
            },
        );
        self.cff_session = Some(session);
        let program = result.map_err(|e| match failure {
            Some(e) => BookV2CffSubsetError::Budget(e),
            None => BookV2CffSubsetError::Font(e),
        })?;
        self.closure_charge(0, 0, 1)?;
        let mut fingerprint = sha256(BOOK_V2_CFF_SUBSET_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &source.fingerprint())?;
        fingerprint = self.fold(fingerprint, &program.fingerprint())?;
        Ok(BookV2CffSubset {
            source,
            program,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
}
