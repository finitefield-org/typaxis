//! Host bytes and all currently observed CFF admission phases use one Source loan.
use super::{BookV2ObservedAccountingError, BookV2ObservedSourceAccounting};
use typaxis_core::NodeId;
use typaxis_font::{CffAdmissionBudgetV2, CffBaseCmapBudgetV2, CffCmapBudgetV2};
use typaxis_resource_admission::{HostReadBudget, HostReadRequest, HostReadStage};
use typaxis_resource_admission::{
    ProductionResourceLedgerBudgetV3, ProductionResourceLedgerPointV3,
    ProductionResourceLedgerStorageV3,
};

pub struct BookV2ObservedResourceAdmissionBudget<'call, 'id, 'cells> {
    accounting: &'call mut BookV2ObservedSourceAccounting<'id, 'cells>,
    owner: NodeId,
}
impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    pub fn resource_admission_budget(
        &mut self,
        owner: NodeId,
    ) -> BookV2ObservedResourceAdmissionBudget<'_, 'id, 'cells> {
        BookV2ObservedResourceAdmissionBudget {
            accounting: self,
            owner,
        }
    }
}
impl HostReadBudget for BookV2ObservedResourceAdmissionBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    const OBSERVE_ATTEMPTS: bool = true;
    const OBSERVE_DECLARATIONS: bool = true;
    const OBSERVE_RESOLUTION: bool = true;
    const OBSERVE_OPEN: bool = true;
    const OBSERVE_OPEN_IDENTITIES: bool = true;
    const OBSERVE_LEDGER: bool = true;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), Self::Error> {
        self.accounting.debit(
            self.owner,
            [
                if request.stage == HostReadStage::OpenIdentityStorage {
                    request.buffer_bytes
                        / typaxis_resource_admission::HOST_OPEN_IDENTITY_NODE_BYTES as u64
                } else {
                    u64::from(
                        matches!(
                            request.stage,
                            HostReadStage::Buffer
                                | HostReadStage::AttemptStorage
                                | HostReadStage::DeclarationUri
                                | HostReadStage::OpenOwner
                        ) || (matches!(
                            request.stage,
                            HostReadStage::RevalidatePath
                                | HostReadStage::OpenPath
                                | HostReadStage::LedgerCandidatePath
                                | HostReadStage::LedgerOpenedPath
                                | HostReadStage::LedgerAliasPath
                        ) && request.buffer_bytes > 0),
                    )
                },
                request.buffer_bytes,
                request.work,
            ],
        )
    }
}
impl typaxis_resource_admission::SafeSvgSourceBudget
    for BookV2ObservedResourceAdmissionBudget<'_, '_, '_>
{
    type Error = BookV2ObservedAccountingError;
    const OBSERVE_COUNT: bool = true;
    const OBSERVE_CLIP_CLOSURE: bool = true;
    const OBSERVE_IR: bool = true;
    const OBSERVE_BINDING: bool = true;
    fn work(
        &mut self,
        request: typaxis_resource_admission::SafeSvgSourceRequest,
    ) -> Result<(), Self::Error> {
        self.accounting.debit(
            self.owner,
            [request.records, request.storage_bytes, request.work],
        )
    }
}
impl ProductionResourceLedgerBudgetV3 for BookV2ObservedResourceAdmissionBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(
        &mut self,
        _: ProductionResourceLedgerStorageV3,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error> {
        let rows = u64::try_from(rows)
            .map_err(|_| BookV2ObservedAccountingError::Records { owner: self.owner })?;
        let bytes = u64::try_from(bytes)
            .map_err(|_| BookV2ObservedAccountingError::Spool { owner: self.owner })?;
        self.accounting.debit(self.owner, [rows, bytes, 0])
    }
    fn work(&mut self, _: ProductionResourceLedgerPointV3, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
}
impl CffBaseCmapBudgetV2 for BookV2ObservedResourceAdmissionBudget<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(
        &mut self,
        s: typaxis_font::CffBaseCmapStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.accounting
            .font_jstf_budget(self.owner)
            .reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
}
impl CffCmapBudgetV2 for BookV2ObservedResourceAdmissionBudget<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.accounting
            .font_jstf_budget(self.owner)
            .reserve_variation(s, r, b)
    }
}
impl CffAdmissionBudgetV2 for BookV2ObservedResourceAdmissionBudget<'_, '_, '_> {
    const OBSERVE_METADATA: bool = true;
    const OBSERVE_TABLES: bool = true;
    const OBSERVE_PROGRAM: bool = true;
    const OBSERVE_OPTIONAL_LOCAL: bool = true;
    const OBSERVE_MATH: bool = true;
    const OBSERVE_JSTF: bool = true;
    const OBSERVE_SOURCE_OWNER: bool = true;
    const OBSERVE_RESOURCE_MEDIA: bool = true;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.accounting
            .font_jstf_budget(self.owner)
            .reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.accounting
            .font_jstf_budget(self.owner)
            .reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
    fn reserve_program(
        &mut self,
        s: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.accounting
            .font_jstf_budget(self.owner)
            .reserve_program(s, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
    fn work_optional_local(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
    fn work_math(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
    fn work_jstf(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, n)
    }
}

impl typaxis_resource_admission::ResourceCatalogBudget
    for BookV2ObservedResourceAdmissionBudget<'_, '_, '_>
{
    type Error = BookV2ObservedAccountingError;
    fn charge(
        &mut self,
        request: typaxis_resource_admission::ResourceCatalogRequest,
    ) -> Result<(), Self::Error> {
        self.accounting.debit(
            self.owner,
            [request.records, request.storage_bytes, request.work],
        )
    }
}
