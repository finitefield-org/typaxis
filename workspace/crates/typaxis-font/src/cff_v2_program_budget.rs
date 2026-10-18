//! Program admission callbacks borrow the caller's cells; no resource authority.
use super::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffProgramStorageV2 {
    IndexObjects,
    DictOperands,
    DictEntries,
    RealText,
    FontDicts,
    StructureSpans,
    FdByGlyph,
    CidByGlyph,
    SidByGlyph,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffProgramStageV2 {
    Owner,
    Header,
    NameMetadata,
    IndexHeader,
    IndexOffset,
    DictToken,
    DictNumber,
    DictDuplicate,
    DictLookup,
    NumericValidation,
    Charstrings,
    FontDict,
    PrivateDict,
    FdSelectHeader,
    FdSelectRange,
    FdSelectFill,
    CharsetHeader,
    CharsetRange,
    CharsetFill,
    CidCount,
    SpanSort,
    SpanOverlap,
    ImplicitFontDict,
    PredefinedCharset,
    SidCount,
    EncodingHeader,
    EncodingRange,
    EncodingCode,
    EncodingSupplement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CffProgramWorkPointV2 {
    pub stage: CffProgramStageV2,
    pub table_offset: Option<u64>,
    pub file_offset: Option<u64>,
    pub fd: Option<u16>,
    pub operator: Option<u16>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffProgramBudgetFailureV2<E> {
    pub error: E,
    pub point: CffProgramWorkPointV2,
}
impl<E> CffProgramBudgetFailureV2<E> {
    pub(super) fn bind_file_offset(&mut self, base: usize) {
        self.point.file_offset = self
            .point
            .table_offset
            .and_then(|n| (base as u64).checked_add(n));
    }
}
pub type CffProgramBudgetResultV2<T, E> =
    std::result::Result<std::result::Result<T, CffProgramErrorV2>, CffProgramBudgetFailureV2<E>>;
#[derive(Debug)]
pub(super) enum Failure<E> {
    Native(CffProgramErrorV2),
    Budget(CffProgramBudgetFailureV2<E>),
}
impl<E> From<CffProgramErrorV2> for Failure<E> {
    fn from(e: CffProgramErrorV2) -> Self {
        Self::Native(e)
    }
}
impl<E> Failure<E> {
    pub(super) fn fd(self, fd: u16) -> Self {
        match self {
            Self::Native(mut e) => {
                e.fd = Some(fd);
                Self::Native(e)
            }
            Self::Budget(mut e) => {
                e.point.fd = Some(fd);
                Self::Budget(e)
            }
        }
    }
}
pub(super) type Result<T, E> = std::result::Result<T, Failure<E>>;
pub(super) struct State<'a, B> {
    pub(super) budget: &'a mut B,
    pub(super) fd: Option<u16>,
    pub(super) source_length: Option<usize>,
    #[cfg(test)]
    allocation_count: usize,
    #[cfg(test)]
    allocation_failure: Option<usize>,
}
impl<'a, B: CffAdmissionBudgetV2> State<'a, B> {
    pub(super) fn new(budget: &'a mut B) -> Self {
        Self {
            budget,
            fd: None,
            source_length: None,
            #[cfg(test)]
            allocation_count: 0,
            #[cfg(test)]
            allocation_failure: None,
        }
    }
    #[cfg(test)]
    fn refuses_native(&mut self) -> bool {
        let at = self.allocation_count;
        self.allocation_count += 1;
        self.allocation_failure == Some(at)
    }
    pub(super) fn string(
        &mut self,
        out: &mut String,
        count: usize,
        at: usize,
    ) -> Result<(), B::Error> {
        self.reserve_bytes(CffProgramStorageV2::RealText, 0, count, at)?;
        #[cfg(test)]
        if self.refuses_native() {
            assert!(out.try_reserve_exact(usize::MAX).is_err());
            return Err(fail(K::AllocationFailure, at).into());
        }
        out.try_reserve_exact(count)
            .map_err(|_| Failure::Native(fail(K::AllocationFailure, at)))
    }

    pub(super) fn work(
        &mut self,
        stage: CffProgramStageV2,
        at: Option<usize>,
        operator: Option<u16>,
        amount: u64,
    ) -> Result<(), B::Error> {
        if !B::OBSERVE_PROGRAM || amount == 0 {
            return Ok(());
        }
        self.budget.work_program(amount).map_err(|error| {
            Failure::Budget(CffProgramBudgetFailureV2 {
                error,
                point: CffProgramWorkPointV2 {
                    stage,
                    table_offset: at
                        .filter(|n| self.source_length.map_or(true, |len| *n <= len))
                        .map(|n| n as u64),
                    file_offset: None,
                    fd: self.fd,
                    operator,
                },
            })
        })
    }
    pub(super) fn reserve<T>(
        &mut self,
        out: &mut Vec<T>,
        count: usize,
        site: CffProgramStorageV2,
        at: usize,
    ) -> Result<(), B::Error> {
        self.reserve_bytes(
            site,
            count,
            count
                .checked_mul(std::mem::size_of::<T>())
                .ok_or_else(|| fail(K::AllocationFailure, at))?,
            at,
        )?;
        #[cfg(test)]
        if self.refuses_native() {
            assert!(out.try_reserve_exact(usize::MAX).is_err());
            return Err(fail(K::AllocationFailure, at).into());
        }
        out.try_reserve_exact(count)
            .map_err(|_| Failure::Native(fail(K::AllocationFailure, at)))
    }
    pub(super) fn reserve_bytes(
        &mut self,
        site: CffProgramStorageV2,
        rows: usize,
        bytes: usize,
        at: usize,
    ) -> Result<(), B::Error> {
        if !B::OBSERVE_PROGRAM {
            return Ok(());
        }
        self.budget
            .reserve_program(site, rows, bytes)
            .map_err(|error| {
                Failure::Budget(CffProgramBudgetFailureV2 {
                    error,
                    point: CffProgramWorkPointV2 {
                        stage: match site {
                            CffProgramStorageV2::IndexObjects => CffProgramStageV2::IndexHeader,
                            CffProgramStorageV2::DictOperands
                            | CffProgramStorageV2::DictEntries => CffProgramStageV2::DictToken,
                            CffProgramStorageV2::RealText => CffProgramStageV2::DictNumber,
                            CffProgramStorageV2::FontDicts => CffProgramStageV2::FontDict,
                            CffProgramStorageV2::StructureSpans => CffProgramStageV2::SpanSort,
                            CffProgramStorageV2::FdByGlyph => CffProgramStageV2::FdSelectHeader,
                            CffProgramStorageV2::CidByGlyph | CffProgramStorageV2::SidByGlyph => {
                                CffProgramStageV2::CharsetHeader
                            }
                        },
                        table_offset: self
                            .source_length
                            .filter(|len| {
                                at <= *len && !(site == CffProgramStorageV2::SidByGlyph && at <= 2)
                            })
                            .map(|_| at as u64),
                        file_offset: None,
                        fd: self.fd,
                        operator: None,
                    },
                })
            })
    }
}
pub(super) fn finish<T, E>(r: Result<T, E>) -> CffProgramBudgetResultV2<T, E> {
    match r {
        Ok(v) => Ok(Ok(v)),
        Err(Failure::Native(e)) => Ok(Err(e)),
        Err(Failure::Budget(e)) => Err(e),
    }
}
// Fallible in-place heapsort: no hidden allocator or infallible comparator loan.
pub(super) fn sort_spans<B: CffAdmissionBudgetV2>(
    v: &mut [(ProgramSpan, u8)],
    s: &mut State<'_, B>,
) -> Result<(), B::Error> {
    fn sift<B: CffAdmissionBudgetV2>(
        v: &mut [(ProgramSpan, u8)],
        mut root: usize,
        end: usize,
        s: &mut State<'_, B>,
    ) -> Result<(), B::Error> {
        loop {
            let left = root * 2 + 1;
            if left >= end {
                return Ok(());
            }
            let mut child = left;
            if left + 1 < end {
                s.work(CffProgramStageV2::SpanSort, None, None, 1)?;
                if v[left] < v[left + 1] {
                    child = left + 1;
                }
            }
            s.work(CffProgramStageV2::SpanSort, None, None, 1)?;
            if v[root] >= v[child] {
                return Ok(());
            }
            s.work(CffProgramStageV2::SpanSort, None, None, 1)?;
            v.swap(root, child);
            root = child;
        }
    }
    for root in (0..v.len() / 2).rev() {
        sift(v, root, v.len(), s)?;
    }
    for end in (1..v.len()).rev() {
        s.work(CffProgramStageV2::SpanSort, None, None, 1)?;
        v.swap(0, end);
        sift(v, 0, end, s)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "cff_v2_program_budget_tests.rs"]
mod tests;
