use super::observed_ledger::{point, reserve, work};
use super::*;
use typaxis_font::{
    FontFamilyAdmissionBudget, FontFamilyAdmissionError, FontFamilyAdmissionStage as FamilyStage,
    FontFamilyStorage,
};
use ProductionResourceLedgerStageV3 as Stage;

struct Budget<'a, B>(&'a mut B);
impl<B: ProductionResourceLedgerBudgetV3> FontFamilyAdmissionBudget for Budget<'_, B> {
    type Error = ProductionResourceLedgerErrorV3<B::Error>;
    fn reserve(
        &mut self,
        site: FontFamilyStorage,
        face: Option<FontFaceId>,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error> {
        let (storage, stage) = match site {
            FontFamilyStorage::Entries => (
                ProductionResourceLedgerStorageV3::FamilyEntries,
                Stage::FamilyEntries,
            ),
            FontFamilyStorage::NameBytes => (
                ProductionResourceLedgerStorageV3::FamilyNames,
                Stage::FamilyNameCopy,
            ),
        };
        reserve(self.0, storage, rows, bytes, point(stage, face, None))
    }
    fn work(
        &mut self,
        stage: FamilyStage,
        face: FontFaceId,
        amount: u64,
    ) -> Result<(), Self::Error> {
        let stage = match stage {
            FamilyStage::Validate => Stage::FamilyValidate,
            FamilyStage::Lookup => Stage::FamilyLookup,
            FamilyStage::Entries => Stage::FamilyEntries,
            FamilyStage::NameCopy => Stage::FamilyNameCopy,
            FamilyStage::Insert => Stage::FamilyInsert,
        };
        work(self.0, point(stage, Some(face), None), amount)
    }
}
pub(super) fn families<B: ProductionResourceLedgerBudgetV3>(
    declarations: &[FontFaceDeclaration],
    budget: &mut B,
) -> Result<FontFamilyTable, ProductionResourceLedgerErrorV3<B::Error>> {
    if !B::OBSERVED {
        return FontFamilyTable::new(
            declarations
                .iter()
                .map(|d| (d.family.clone(), d.font_face_id))
                .collect(),
        )
        .map_err(|e| map_font_family_error(e).into());
    }
    FontFamilyTable::try_from_borrowed_with_budget(
        declarations
            .iter()
            .map(|d| (d.family.as_str(), d.font_face_id)),
        &mut Budget(budget),
    )
    .map_err(|e| match e {
        FontFamilyAdmissionError::Refused { error, .. } => error,
        FontFamilyAdmissionError::Family(error) => map_font_family_error(error).into(),
        FontFamilyAdmissionError::Capacity | FontFamilyAdmissionError::ArithmeticOverflow => {
            ResourceAdmissionError::ResourceLimit.into()
        }
        FontFamilyAdmissionError::IteratorLength => {
            ResourceAdmissionError::ReceiptIdentityMismatch.into()
        }
    })
}
