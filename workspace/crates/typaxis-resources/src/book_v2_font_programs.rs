//! Complete original-format programs for all fonts of one book display.
use super::*;
pub const BOOK_V2_FONT_PROGRAMS_ALGORITHM: &str = "typaxis.book-2-font-programs/1";
#[derive(Debug)]
pub enum BookV2FontProgramsError {
    Budget(BookV2FontSelectionError),
    Evaluation(BookV2CffProgramError),
    TrueType(BookV2TrueTypeSubsetError),
    Cff(BookV2CffSubsetError),
}
impl From<E> for BookV2FontProgramsError {
    fn from(e: E) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2FontProgramsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 font programs: {self:?}")
    }
}
impl std::error::Error for BookV2FontProgramsError {}
enum Program<'x, 'c, 'a> {
    TrueType(BookV2TrueTypeSubset<'x, 'c, 'a>),
    Cff(BookV2CffSubset<'x, 'c, 'a>),
}
pub struct BookV2FontProgram<'x, 'c, 'a>(Program<'x, 'c, 'a>);
impl<'x, 'c, 'a> BookV2FontProgram<'x, 'c, 'a> {
    pub fn source(&self) -> &'x BookV2ClosedFont<'c, 'a> {
        match &self.0 {
            Program::TrueType(p) => p.source(),
            Program::Cff(p) => p.source(),
        }
    }
    pub fn kind(&self) -> BookV2FontClosureKind {
        self.source().kind()
    }
    pub fn bytes(&self) -> &[u8] {
        match &self.0 {
            Program::TrueType(p) => p.bytes(),
            Program::Cff(p) => p.program().bytes(),
        }
    }
    pub fn sha256(&self) -> [u8; 32] {
        match &self.0 {
            Program::TrueType(p) => p.sha256(),
            Program::Cff(p) => p.program().sha256(),
        }
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        match &self.0 {
            Program::TrueType(p) => p.fingerprint(),
            Program::Cff(p) => p.fingerprint(),
        }
    }
    pub fn postscript_name(&self) -> &str {
        match &self.0 {
            Program::TrueType(p) => p.postscript_name(),
            Program::Cff(p) => p.program().postscript_name(),
        }
    }
    /// Original units; use the actual admitted source font's units_per_em.
    pub fn advance(&self, gid: OriginalGlyphId) -> Option<u16> {
        match &self.0 {
            Program::TrueType(p) => p.original_widths().get(&gid).copied(),
            Program::Cff(p) => p.program().original_widths().get(&gid).copied(),
        }
    }
    pub fn subset_gid(&self, gid: OriginalGlyphId) -> Option<SubsetGlyphId> {
        self.source().subset_gid(gid)
    }
    pub fn metrics(&self) -> crate::PdfFontMetrics {
        match &self.0 {
            Program::TrueType(p) => p.metrics().clone(),
            Program::Cff(p) => {
                let m = p.program().metrics();
                crate::PdfFontMetrics {
                    ascent_1000: m.ascent_1000,
                    descent_1000: m.descent_1000,
                    cap_height_1000: m.cap_height_1000,
                    stem_v_1000: m.stem_v_1000,
                    italic_angle_fixed_16_16: m.italic_angle_fixed_16_16,
                    flags: m.flags,
                    bbox_1000: m.bbox_1000,
                }
            }
        }
    }
}
pub struct BookV2FontPrograms<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'x BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    fonts: Vec<BookV2FontProgram<'x, 'c, 'a>>,
    fingerprint: [u8; 32],
    records: u64,
    spool: u64,
    work: u64,
}
impl<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontPrograms<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'x BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn fonts(&self) -> &[BookV2FontProgram<'x, 'c, 'a>] {
        &self.fonts
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
    pub fn write_font_programs<'x, 'c>(
        &mut self,
        closures: &'x BookV2FontClosures<'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        BookV2FontPrograms<'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2FontProgramsError,
    > {
        self.prepare_cff_programs(closures).map_err(|e| match e {
            BookV2CffProgramError::Budget(e) => BookV2FontProgramsError::Budget(e),
            e => BookV2FontProgramsError::Evaluation(e),
        })?;
        let n = closures.fonts().len();
        self.closure_charge(
            n.checked_add(1).ok_or(E::Records)?,
            n.checked_mul(std::mem::size_of::<BookV2FontProgram<'x, 'c, 'a>>())
                .ok_or(E::Spool)?,
            1,
        )?;
        let mut fonts = Vec::new();
        fonts.try_reserve_exact(n).map_err(|_| E::Allocation)?;
        let mut fingerprint = sha256(BOOK_V2_FONT_PROGRAMS_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &closures.fingerprint())?;
        for (index, source) in closures.fonts().iter().enumerate() {
            self.step()?;
            let program = match source.kind() {
                BookV2FontClosureKind::TrueType => Program::TrueType(
                    self.write_truetype_font(closures, index)
                        .map_err(|e| match e {
                            BookV2TrueTypeSubsetError::Budget(e) => {
                                BookV2FontProgramsError::Budget(e)
                            }
                            e => BookV2FontProgramsError::TrueType(e),
                        })?,
                ),
                BookV2FontClosureKind::Cff1V2 => {
                    Program::Cff(self.write_cff_font(closures, index).map_err(|e| match e {
                        BookV2CffSubsetError::Budget(e) => BookV2FontProgramsError::Budget(e),
                        e => BookV2FontProgramsError::Cff(e),
                    })?)
                }
            };
            let font = BookV2FontProgram(program);
            fingerprint = self.fold(fingerprint, &font.fingerprint())?;
            fonts.push(font);
        }
        Ok(BookV2FontPrograms {
            source: closures,
            fonts,
            fingerprint,
            records: self.records,
            spool: self.spool,
            work: self.work,
        })
    }
}
