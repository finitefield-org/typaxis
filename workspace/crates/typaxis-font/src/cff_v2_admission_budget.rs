//! Observed SFNT, cmap and opt-in font metadata phases.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffSfntStorageV2 {
    DirectoryRecords,
    OrderedRecords,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffSfntStageV2 {
    Header,
    Directory,
    TableChecksum,
    RequiredTables,
    DirectoryCopy,
    DirectoryOrder,
    Padding,
    FontChecksum,
    DeclaredMediaHeader,
    DeclaredMediaRecord,
    DeclaredMediaDiagnosticHeader,
    DeclaredMediaDiagnosticFace,
    DeclaredMediaDiagnosticRecord,
    FontForgeMetadata,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CffSfntWorkPointV2 {
    pub stage: CffSfntStageV2,
    pub table_tag: Option<[u8; 4]>,
    pub file_offset: Option<u64>,
    pub table_offset: Option<u64>,
}
pub trait CffAdmissionBudgetV2: CffCmapBudgetV2 {
    const OBSERVE_SFNT: bool = true;
    /// Resource resolver outline attestation and its error-only diagnostics.
    /// Direct font APIs and existing phase-only adapters keep their billing.
    const OBSERVE_RESOURCE_MEDIA: bool = false;
    /// Opt in separately so existing SFNT/cmap callers retain their billing.
    const OBSERVE_METADATA: bool = false;
    /// Fixed table decoders, vertical validation and bounded directory lookups.
    /// Optional-table library internals and CFF program inspection are separate.
    const OBSERVE_TABLES: bool = false;
    const OBSERVE_PROGRAM: bool = false;
    /// Local allocation-free BASE/GDEF/kern grammar; library walks are separate.
    const OBSERVE_OPTIONAL_LOCAL: bool = false;
    const OBSERVE_MATH: bool = false;
    /// JSTF grammar and its GPOS intersection walks, separately versioned billing.
    const OBSERVE_JSTF: bool = false;
    /// Opt in only for owned-buffer APIs; uses the metadata reserve/work hooks
    /// before creating the small shared owner. Slice APIs never add this fee.
    const OBSERVE_SOURCE_OWNER: bool = false;
    fn reserve_sfnt(
        &mut self,
        site: CffSfntStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error>;
    fn work_sfnt(&mut self, amount: u64) -> Result<(), Self::Error>;
    fn reserve_metadata(
        &mut self,
        _: CffMetadataStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("metadata observation was not enabled")
    }
    fn work_metadata(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("metadata observation was not enabled")
    }
    fn work_tables(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("table observation was not enabled")
    }
    fn reserve_program(
        &mut self,
        _: CffProgramStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("program observation was not enabled")
    }
    fn work_math(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("MATH observation was not enabled")
    }
    fn work_jstf(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("JSTF observation was not enabled")
    }
    fn work_optional_local(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("local optional observation was not enabled")
    }
    fn work_program(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("program observation was not enabled")
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffTableAdmissionStageV2 {
    DirectoryIndex,
    TableLookup,
    ContextLookup,
    ErrorContext,
    FixedDecoder,
    EmbeddingPermission,
    VerticalHeader,
    VerticalRecord,
    OptionalDispatch,
    OptionalLocalHeader,
    OptionalLocalField,
    OptionalLocalRange,
    OptionalLocalOffset,
    MathHeader,
    MathField,
    MathOffset,
    JstfHeader,
    JstfField,
    JstfRange,
    JstfOffset,
    JstfReader,
    JstfVisit,
    JstfSetInit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CffTableAdmissionWorkPointV2 {
    pub stage: CffTableAdmissionStageV2,
    pub table_tag: Option<[u8; 4]>,
    pub file_offset: Option<u64>,
    pub table_offset: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffTableAdmissionBudgetFailureV2<E> {
    pub error: E,
    pub point: CffTableAdmissionWorkPointV2,
}
impl<E> CffTableAdmissionBudgetFailureV2<E> {
    pub(in crate::cff) fn bind_file_offset(&mut self, records: &[TableRecord]) {
        if self.point.file_offset.is_none() {
            if let (Some(tag), Some(offset)) = (self.point.table_tag, self.point.table_offset) {
                self.point.file_offset = records
                    .iter()
                    .find(|r| r.tag == tag)
                    .and_then(|r| (r.offset as u64).checked_add(offset));
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffMetadataStorageV2 {
    SharedSourceOwner,
    HorizontalAdvances,
    HorizontalBearings,
    FamilyName,
    SubfamilyName,
    PostscriptName,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffMetadataStageV2 {
    SourceOwner,
    HorizontalHeader,
    HorizontalMetrics,
    NameHeader,
    NameRecord,
    NameValidation,
    NameSelection,
    NameCopy,
    PostscriptValidation,
    SourceDigest,
    IdentityAssembly,
    IdentityDigest,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CffMetadataWorkPointV2 {
    pub stage: CffMetadataStageV2,
    pub table_tag: Option<[u8; 4]>,
    pub file_offset: Option<u64>,
    pub table_offset: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffMetadataBudgetFailureV2<E> {
    pub error: E,
    pub point: CffMetadataWorkPointV2,
}
impl<E> CffMetadataBudgetFailureV2<E> {
    pub(in crate::cff) fn bind_file_offset(&mut self, base: usize) {
        self.point.file_offset = self
            .point
            .table_offset
            .and_then(|n| (base as u64).checked_add(n));
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffSfntBudgetFailureV2<E> {
    pub error: E,
    pub point: CffSfntWorkPointV2,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CffAdmissionBudgetFailureV2<E> {
    Sfnt(CffSfntBudgetFailureV2<E>),
    Cmap(CffCmapBudgetFailureV2<E>),
    Metadata(CffMetadataBudgetFailureV2<E>),
    Tables(CffTableAdmissionBudgetFailureV2<E>),
    Program(CffProgramBudgetFailureV2<E>),
}
impl<E> CffAdmissionBudgetFailureV2<E> {
    pub(super) fn cmap_only(self) -> CffCmapBudgetFailureV2<E> {
        match self {
            Self::Cmap(e) => e,
            Self::Sfnt(_) => unreachable!("cmap-only observation cannot refuse SFNT work"),
            Self::Metadata(_) => unreachable!("cmap-only observation cannot refuse metadata"),
            Self::Tables(_) => unreachable!("cmap-only observation cannot refuse table work"),
            Self::Program(_) => unreachable!("cmap-only observation cannot refuse program work"),
        }
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for CffAdmissionBudgetFailureV2<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 admission budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CffAdmissionBudgetFailureV2<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sfnt(e) => Some(&e.error),
            Self::Cmap(e) => Some(e),
            Self::Metadata(e) => Some(&e.error),
            Self::Tables(e) => Some(&e.error),
            Self::Program(e) => Some(&e.error),
        }
    }
}
pub type CffAdmissionBudgetResultV2<T, F, E> = Result<Result<T, F>, CffAdmissionBudgetFailureV2<E>>;

pub(super) struct CmapOnly<'a, B>(pub &'a mut B);
impl<B: CffCmapBudgetV2> CffBaseCmapBudgetV2 for CmapOnly<'_, B> {
    type Error = B::Error;
    const OBSERVED: bool = B::OBSERVED;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl<B: CffCmapBudgetV2> CffCmapBudgetV2 for CmapOnly<'_, B> {
    const OBSERVE_VARIATIONS: bool = B::OBSERVE_VARIATIONS;
    fn reserve_variation(
        &mut self,
        s: CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl<B: CffCmapBudgetV2> CffAdmissionBudgetV2 for CmapOnly<'_, B> {
    const OBSERVE_SFNT: bool = false;
    fn reserve_sfnt(&mut self, _: CffSfntStorageV2, _: usize, _: usize) -> Result<(), Self::Error> {
        unreachable!("cmap-only SFNT reserve")
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("cmap-only SFNT work")
    }
}
pub(in crate::cff) struct UnobservedAdmission;
impl CffBaseCmapBudgetV2 for UnobservedAdmission {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), Self::Error> {
        unreachable!("ordinary reserve")
    }
    fn work(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("ordinary work")
    }
}
impl CffCmapBudgetV2 for UnobservedAdmission {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary variation reserve")
    }
}
impl CffAdmissionBudgetV2 for UnobservedAdmission {
    const OBSERVE_SFNT: bool = false;
    fn reserve_sfnt(&mut self, _: CffSfntStorageV2, _: usize, _: usize) -> Result<(), Self::Error> {
        unreachable!("ordinary SFNT reserve")
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("ordinary SFNT work")
    }
}
