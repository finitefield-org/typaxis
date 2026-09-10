//! One aggregate Type2 session for the actual closed book fonts. Statistics
//! are observations, not font/PDF receipts or externally supplied authority.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2CffProgramStatistics {
    pub operations_used: u64,
    pub outline_segments_used: u64,
    pub cached_glyphs: usize,
}
#[derive(Debug)]
pub enum BookV2CffProgramError {
    Budget(BookV2FontSelectionError),
    Cff(typaxis_font::CffSelectionFailureV2),
}
impl From<E> for BookV2CffProgramError {
    fn from(e: E) -> Self {
        Self::Budget(e)
    }
}
impl std::fmt::Display for BookV2CffProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 CFF programs: {self:?}")
    }
}
impl std::error::Error for BookV2CffProgramError {}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2FontSelectionBuilder<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn cff_program_statistics(&self) -> BookV2CffProgramStatistics {
        // The session is only taken during an exclusive mutable method call.
        let session = self.cff_session.as_ref().expect("owned CFF session");
        BookV2CffProgramStatistics {
            operations_used: session.operations_used(),
            outline_segments_used: session.outline_segments_used(),
            cached_glyphs: session.cached_glyph_count(),
        }
    }
    pub fn prepare_cff_programs(
        &mut self,
        closures: &BookV2FontClosures<'_, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2CffProgramStatistics, BookV2CffProgramError> {
        if !std::ptr::eq(self.display, closures.selection().display()) {
            return Err(E::Identity.into());
        }
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
        // This actual admitted instance table has exactly one instance per
        // face, in face-ID order. Check the whole sequence before evaluation;
        // same-source aliases at different face IDs share the session cache.
        let mut previous = None;
        for font in closures.fonts() {
            self.step()?;
            let face = font.source().instance().font().font_face_id();
            if previous.is_some_and(|p| p >= face) {
                return Err(E::Identity.into());
            }
            previous = Some(face);
        }
        let mut session = self.cff_session.take().ok_or(E::Identity)?;
        let result: Result<(), BookV2CffProgramError> = (|| {
            for font in closures.fonts() {
                self.step()?;
                let Closure::Cff(closed) = &font.closure else {
                    continue;
                };
                let AdmittedProductionFontV3::Cff1V2(source) = font.source().instance().font()
                else {
                    return Err(E::Identity.into());
                };
                let mut failure = None;
                let result = session.prepare_closure_with_charge(
                    source.admission(),
                    closed,
                    &mut |records, bytes, work| {
                        self.closure_charge(records, bytes, work).map_err(|e| {
                            failure = Some(e);
                            typaxis_font::Cff1Error::SubsetByteLimit
                        })
                    },
                );
                result.map_err(|e| match failure {
                    Some(e) => BookV2CffProgramError::Budget(e),
                    None => BookV2CffProgramError::Cff(e),
                })?;
            }
            Ok(())
        })();
        // Preserve successfully cached glyphs and failed evaluation counters
        // on every error. Retrying cannot reset Type2 or document budgets.
        self.cff_session = Some(session);
        result?;
        Ok(self.cff_program_statistics())
    }
}
