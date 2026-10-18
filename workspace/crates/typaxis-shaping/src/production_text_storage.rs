//! Requested body/region scratch uses the same Source loan as retained output.
//! This meters requested storage, not allocator capacity or a process RSS bound.
use super::*;
use book_v2::{BookV2ObservedAccountingError, BookV2ObservedSourceAccounting};
use typaxis_resource_admission::{
    AdmittedProductionFontInstancesV3, AdmittedProductionResourceLedgerV3,
    ProductionFontInstanceBudgetV3, ProductionFontInstanceErrorV3,
};

pub(super) enum Error {
    Accounting(BookV2ObservedAccountingError),
    Shape(ProductionTextShapeError),
}

struct InstanceStorage<'call, 'id, 'cells> {
    accounting: &'call mut BookV2ObservedSourceAccounting<'id, 'cells>,
    owner: NodeId,
}
impl ProductionFontInstanceBudgetV3 for InstanceStorage<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve_faces(&mut self, count: usize) -> Result<(), Self::Error> {
        let records =
            u64::try_from(count).map_err(|_| Self::Error::Records { owner: self.owner })?;
        let bytes = count
            .checked_mul(std::mem::size_of::<FontFaceId>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(Self::Error::Spool { owner: self.owner })?;
        self.accounting.debit(self.owner, [records, bytes, records])
    }
    fn work(&mut self, amount: u64) -> Result<(), Self::Error> {
        self.accounting.debit_work(self.owner, amount)
    }
}

pub(super) fn instances<'a>(
    ledger: &'a AdmittedProductionResourceLedgerV3,
    owner: NodeId,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<AdmittedProductionFontInstancesV3<'a>, Error> {
    AdmittedProductionFontInstancesV3::from_all_faces_with_budget(
        ledger,
        &mut InstanceStorage { accounting, owner },
    )
    .map_err(|e| match e {
        ProductionFontInstanceErrorV3::Budget(e) => Error::Accounting(e),
        ProductionFontInstanceErrorV3::AllocationFailure => Error::Shape(error(
            owner,
            ProductionTextShapeErrorKind::AllocationFailure,
        )),
        ProductionFontInstanceErrorV3::ResourceLimit
        | ProductionFontInstanceErrorV3::InvalidFaceOrder => {
            Error::Shape(error(owner, ProductionTextShapeErrorKind::ReceiptMismatch))
        }
    })
}

struct BodyStorage<'call, 'id, 'cells> {
    accounting: &'call mut BookV2ObservedSourceAccounting<'id, 'cells>,
    owner: NodeId,
    error: Option<BookV2ObservedAccountingError>,
}
impl BodyStorage<'_, '_, '_> {
    fn accepted(
        &mut self,
        result: Result<(), BookV2ObservedAccountingError>,
    ) -> Result<(), LinkedShaperError> {
        result.map_err(|error| {
            self.error.get_or_insert(error);
            LinkedShaperError::AllocationFailure
        })
    }
}
impl Storage for BodyStorage<'_, '_, '_> {
    fn owner(&mut self, owner: NodeId) {
        self.owner = owner;
    }
    fn reserve<T>(&mut self, _: Site, count: usize) -> Result<(), LinkedShaperError> {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|n| u64::try_from(n).ok());
        let result = match bytes {
            Some(bytes) => self.accounting.debit(self.owner, [0, bytes, count as u64]),
            None => Err(BookV2ObservedAccountingError::Spool { owner: self.owner }),
        };
        self.accepted(result)
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        // The document's output callback preflights these exact records, and
        // retain_intrinsic_output publishes their accepted prefix once.
        Ok(())
    }
    fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
        let result = self.accounting.debit_work(self.owner, count);
        self.accepted(result)
    }
}

pub(super) fn shape<'a>(
    request: BodyShapeRequest<'a, '_, '_>,
    records: &mut u64,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<BodyShapeOutput<'a>, Error> {
    let mut storage = BodyStorage {
        accounting,
        owner: NodeId::new(0),
        error: None,
    };
    let result = shape_document_with_storage(
        request.flow,
        request.admitted,
        request.limits,
        request.epoch,
        request.lines,
        request.algorithm,
        request.maximum_records,
        records,
        &mut storage,
    );
    if let Some(error) = storage.error {
        return Err(Error::Accounting(error));
    }
    result.map_err(Error::Shape)
}
