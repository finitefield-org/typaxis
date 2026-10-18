//! Base-cmap requests borrow the existing source cells; no admission or graph
//! authority is created by this adapter and no caller reference is retained.
use super::{BookV2ObservedAccountingError, BookV2ObservedSourceAccounting};
use typaxis_core::NodeId;
use typaxis_font::{CffBaseCmapBudgetV2, CffBaseCmapStorageV2};

pub struct BookV2ObservedBaseCmapBudget<'call, 'id, 'cells> {
    accounting: &'call mut BookV2ObservedSourceAccounting<'id, 'cells>,
    owner: NodeId,
}
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn base_cmap_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedBaseCmapBudget<'_, 'id, 'cells> {
        BookV2ObservedBaseCmapBudget {
            accounting: self,
            owner,
        }
    }
}
impl BookV2ObservedBaseCmapBudget<'_, '_, '_> {
    fn reserve_cells(
        &mut self,
        rows: usize,
        bytes: usize,
    ) -> Result<(), BookV2ObservedAccountingError> {
        let r = u64::try_from(rows)
            .map_err(|_| BookV2ObservedAccountingError::Records { owner: self.owner })?;
        let s = u64::try_from(bytes)
            .map_err(|_| BookV2ObservedAccountingError::Spool { owner: self.owner })?;
        self.accounting.debit(self.owner, [r, s, 0])
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedBaseCmapBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(
        &mut self,
        _: CffBaseCmapStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error> {
        self.reserve_cells(rows, bytes)
    }
    fn work(&mut self, amount: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, amount)
    }
}

/// Full cmap phases borrow the same source cells as base-only callers.
pub struct BookV2ObservedCmapBudget<'call, 'id, 'cells>(
    BookV2ObservedBaseCmapBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn cmap_budget(&mut self, owner: NodeId) -> BookV2ObservedCmapBudget<'_, 'id, 'cells> {
        BookV2ObservedCmapBudget(self.base_cmap_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedCmapBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(
        &mut self,
        site: CffBaseCmapStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve(site, rows, bytes)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedCmapBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_cells(rows, bytes)
    }
}

/// SFNT preflight and cmap debit the existing source cells without a new owner.
pub struct BookV2ObservedAdmissionBudget<'call, 'id, 'cells>(
    BookV2ObservedCmapBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn admission_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedAdmissionBudget<'_, 'id, 'cells> {
        BookV2ObservedAdmissionBudget(self.cmap_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedAdmissionBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedAdmissionBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedAdmissionBudget<'_, '_, '_> {
    fn reserve_sfnt(
        &mut self,
        _: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0 .0.reserve_cells(r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}

/// Metrics, names and admission digests use the same command Source cells.
pub struct BookV2ObservedFontMetadataBudget<'call, 'id, 'cells>(
    BookV2ObservedAdmissionBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn font_metadata_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedFontMetadataBudget<'_, 'id, 'cells> {
        BookV2ObservedFontMetadataBudget(self.admission_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedFontMetadataBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedFontMetadataBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedFontMetadataBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        _: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0 .0 .0.reserve_cells(r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}

/// Fixed table decoding and vertical validation borrow the same Source cells.
pub struct BookV2ObservedFontTablesBudget<'call, 'id, 'cells>(
    BookV2ObservedFontMetadataBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn font_tables_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedFontTablesBudget<'_, 'id, 'cells> {
        BookV2ObservedFontTablesBudget(self.font_metadata_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedFontTablesBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedFontTablesBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedFontTablesBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    const OBSERVE_TABLES: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}

/// Program admission uses the existing command Source loan as well.
pub struct BookV2ObservedFontProgramBudget<'call, 'id, 'cells>(
    BookV2ObservedFontTablesBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn font_program_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedFontProgramBudget<'_, 'id, 'cells> {
        BookV2ObservedFontProgramBudget(self.font_tables_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedFontProgramBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedFontProgramBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedFontProgramBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    const OBSERVE_TABLES: bool = true;
    const OBSERVE_PROGRAM: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_tables(n)
    }
    fn reserve_program(
        &mut self,
        _: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0 .0 .0 .0 .0.reserve_cells(r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}

/// Local BASE/GDEF/kern validation borrows the same command Source loan.
pub struct BookV2ObservedFontOptionalLocalBudget<'call, 'id, 'cells>(
    BookV2ObservedFontProgramBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn font_optional_local_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedFontOptionalLocalBudget<'_, 'id, 'cells> {
        BookV2ObservedFontOptionalLocalBudget(self.font_program_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedFontOptionalLocalBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedFontOptionalLocalBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedFontOptionalLocalBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    const OBSERVE_TABLES: bool = true;
    const OBSERVE_PROGRAM: bool = true;
    const OBSERVE_OPTIONAL_LOCAL: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_tables(n)
    }
    fn reserve_program(
        &mut self,
        site: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_program(site, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_program(n)
    }
    fn work_optional_local(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}

/// Shared MATH table validation uses the existing command Source loan.
pub struct BookV2ObservedFontMathBudget<'call, 'id, 'cells>(
    BookV2ObservedFontOptionalLocalBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn font_math_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedFontMathBudget<'_, 'id, 'cells> {
        BookV2ObservedFontMathBudget(self.font_optional_local_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedFontMathBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedFontMathBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedFontMathBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    const OBSERVE_TABLES: bool = true;
    const OBSERVE_PROGRAM: bool = true;
    const OBSERVE_OPTIONAL_LOCAL: bool = true;
    const OBSERVE_MATH: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_tables(n)
    }
    fn reserve_program(
        &mut self,
        site: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_program(site, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_program(n)
    }
    fn work_optional_local(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_optional_local(n)
    }
    fn work_math(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}

/// JSTF grammar and GPOS intersections uses the existing command Source loan.
pub struct BookV2ObservedFontJstfBudget<'call, 'id, 'cells>(
    BookV2ObservedFontMathBudget<'call, 'id, 'cells>,
);
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn font_jstf_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedFontJstfBudget<'_, 'id, 'cells> {
        BookV2ObservedFontJstfBudget(self.font_math_budget(owner))
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedFontJstfBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(&mut self, s: CffBaseCmapStorageV2, r: usize, b: usize) -> Result<(), Self::Error> {
        self.0.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl typaxis_font::CffCmapBudgetV2 for BookV2ObservedFontJstfBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_variation(s, r, b)
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for BookV2ObservedFontJstfBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    const OBSERVE_TABLES: bool = true;
    const OBSERVE_PROGRAM: bool = true;
    const OBSERVE_OPTIONAL_LOCAL: bool = true;
    const OBSERVE_MATH: bool = true;
    const OBSERVE_JSTF: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_tables(n)
    }
    fn reserve_program(
        &mut self,
        site: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve_program(site, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_program(n)
    }
    fn work_optional_local(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_optional_local(n)
    }
    fn work_math(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work_math(n)
    }
    fn work_jstf(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
