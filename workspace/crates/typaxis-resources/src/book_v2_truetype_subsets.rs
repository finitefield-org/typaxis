//! Actual TrueType programs from the closed fonts of an exact book-2 display.
use super::*;

pub const BOOK_V2_TRUETYPE_SUBSET_ALGORITHM: &str = "typaxis.book-2-truetype-subset/1";
#[derive(Debug)]
pub enum BookV2TrueTypeSubsetError {
    Budget(BookV2FontSelectionError),
    WrongFontKind,
    InvalidFontIndex,
    Font(crate::ResourceError),
}
impl From<E> for BookV2TrueTypeSubsetError {
    fn from(e: E) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2TrueTypeSubsetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 TrueType subset: {self:?}")
    }
}
impl std::error::Error for BookV2TrueTypeSubsetError {}

pub struct BookV2TrueTypeSubset<'x, 'c, 'a> {
    source: &'x BookV2ClosedFont<'c, 'a>,
    written: crate::truetype_subset_writer::WrittenTrueTypeSubset,
    name: String,
    hash: [u8; 32],
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'x, 'c, 'a> BookV2TrueTypeSubset<'x, 'c, 'a> {
    pub fn source(&self) -> &'x BookV2ClosedFont<'c, 'a> {
        self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.written.bytes
    }
    pub fn postscript_name(&self) -> &str {
        &self.name
    }
    pub fn sha256(&self) -> [u8; 32] {
        self.hash
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    /// Advances remain in original font units, with the source instance's UPM.
    pub fn original_widths(&self) -> &std::collections::BTreeMap<OriginalGlyphId, u16> {
        &self.written.widths
    }
    pub fn metrics(&self) -> &crate::PdfFontMetrics {
        &self.written.metrics
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
    pub fn write_truetype_font<'x, 'c>(
        &mut self,
        closures: &'x BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        index: usize,
    ) -> Result<BookV2TrueTypeSubset<'x, 'c, 'a>, BookV2TrueTypeSubsetError> {
        if !std::ptr::eq(self.display, closures.selection().display()) {
            return Err(E::Identity.into());
        }
        let source = closures
            .fonts()
            .get(index)
            .ok_or(BookV2TrueTypeSubsetError::InvalidFontIndex)?;
        let Closure::TrueType(prepared) = &source.closure else {
            return Err(BookV2TrueTypeSubsetError::WrongFontKind);
        };
        self.records = self.records.max(closures.record_charge());
        self.spool = self.spool.max(closures.spool_charge());
        self.work = self.work.max(closures.work_steps());
        if self.records > self.maximum_records {
            return Err(E::Records.into());
        }
        if self.spool > self.maximum_spool {
            return Err(E::Spool.into());
        }
        if self.work > self.maximum_work {
            return Err(E::Work.into());
        }
        let instance = source.source().instance().font_instance_id();
        let maximum_bytes = self
            .display
            .admitted()
            .effective_limits()
            .extension()
            .get()
            .max_font_subset_bytes;
        self.closure_charge(1, 14, 1)?;
        let mut failure = None;
        let result = crate::truetype_subset_writer::write(
            prepared,
            instance,
            maximum_bytes,
            &mut |records, bytes, work| {
                self.closure_charge(records, bytes, work).map_err(|e| {
                    failure = Some(e);
                    crate::ResourceError::ResourceLimit
                })
            },
        );
        let written = result.map_err(|e| match failure {
            Some(e) => BookV2TrueTypeSubsetError::Budget(e),
            None => BookV2TrueTypeSubsetError::Font(e),
        })?;
        self.closure_charge(0, 0, written.bytes.len().div_ceil(64) + 1)?;
        let hash = sha256(&written.bytes);
        let name = crate::expected_subset_postscript_name(instance)
            .map_err(BookV2TrueTypeSubsetError::Font)?;
        let mut fingerprint = sha256(BOOK_V2_TRUETYPE_SUBSET_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &source.fingerprint())?;
        fingerprint = self.fold(fingerprint, &hash)?;
        fingerprint = self.fold(fingerprint, &(written.bytes.len() as u64).to_be_bytes())?;
        fingerprint = self.fold(fingerprint, name.as_bytes())?;
        Ok(BookV2TrueTypeSubset {
            source,
            written,
            name,
            hash,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
}
