//! Dense ledger storage and canonicalization on a caller-owned budget.
//! Alias and family stages have separate helpers; earlier admission is separate.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionResourceLedgerStorageV3 {
    Aliases,
    FamilyEntries,
    FamilyNames,
    Fonts,
    Images,
    CanonicalJcs,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionResourceLedgerStageV3 {
    ImageIds,
    Aliases,
    AliasOrder,
    AliasCompare,
    FamilyValidate,
    FamilyLookup,
    FamilyEntries,
    FamilyNameCopy,
    FamilyInsert,
    FontKinds,
    FontConflicts,
    Fonts,
    Images,
    CanonicalCapacity,
    CanonicalFonts,
    CanonicalImages,
    CanonicalHash,
    CanonicalFinish,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionResourceLedgerPointV3 {
    pub stage: ProductionResourceLedgerStageV3,
    pub font_face_id: Option<FontFaceId>,
    pub image_id: Option<ImageResourceId>,
}
pub trait ProductionResourceLedgerBudgetV3 {
    type Error;
    const OBSERVED: bool = true;
    fn reserve(
        &mut self,
        site: ProductionResourceLedgerStorageV3,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error>;
    fn work(
        &mut self,
        point: ProductionResourceLedgerPointV3,
        amount: u64,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub struct ProductionResourceLedgerBudgetFailureV3<E> {
    pub error: E,
    pub point: ProductionResourceLedgerPointV3,
}
impl<E: std::fmt::Debug> std::fmt::Display for ProductionResourceLedgerBudgetFailureV3<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production resource ledger budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error
    for ProductionResourceLedgerBudgetFailureV3<E>
{
}
#[derive(Debug)]
pub enum ProductionResourceLedgerErrorV3<E> {
    Native(ProductionResourceErrorV3),
    Budget(ProductionResourceLedgerBudgetFailureV3<E>),
}
impl<E> From<ResourceAdmissionError> for ProductionResourceLedgerErrorV3<E> {
    fn from(e: ResourceAdmissionError) -> Self {
        Self::Native(e.into())
    }
}
impl<E> From<ProductionResourceErrorV3> for ProductionResourceLedgerErrorV3<E> {
    fn from(e: ProductionResourceErrorV3) -> Self {
        Self::Native(e)
    }
}
pub(super) struct Unobserved;
impl ProductionResourceLedgerBudgetV3 for Unobserved {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn reserve(
        &mut self,
        _: ProductionResourceLedgerStorageV3,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!()
    }
    fn work(&mut self, _: ProductionResourceLedgerPointV3, _: u64) -> Result<(), Self::Error> {
        unreachable!()
    }
}
pub(super) fn point(
    stage: ProductionResourceLedgerStageV3,
    font_face_id: Option<FontFaceId>,
    image_id: Option<ImageResourceId>,
) -> ProductionResourceLedgerPointV3 {
    ProductionResourceLedgerPointV3 {
        stage,
        font_face_id,
        image_id,
    }
}
pub(super) fn work<B: ProductionResourceLedgerBudgetV3>(
    budget: &mut B,
    p: ProductionResourceLedgerPointV3,
    n: u64,
) -> Result<(), ProductionResourceLedgerErrorV3<B::Error>> {
    if B::OBSERVED {
        budget.work(p, n).map_err(|error| {
            ProductionResourceLedgerErrorV3::Budget(ProductionResourceLedgerBudgetFailureV3 {
                error,
                point: p,
            })
        })?;
    }
    Ok(())
}
pub(super) fn reserve<B: ProductionResourceLedgerBudgetV3>(
    budget: &mut B,
    site: ProductionResourceLedgerStorageV3,
    rows: usize,
    bytes: usize,
    p: ProductionResourceLedgerPointV3,
) -> Result<(), ProductionResourceLedgerErrorV3<B::Error>> {
    if B::OBSERVED {
        budget.reserve(site, rows, bytes).map_err(|error| {
            ProductionResourceLedgerErrorV3::Budget(ProductionResourceLedgerBudgetFailureV3 {
                error,
                point: p,
            })
        })?;
    }
    Ok(())
}
pub(super) fn fonts<B: ProductionResourceLedgerBudgetV3>(
    native: BTreeMap<FontFaceId, AdmittedFont>,
    cff: binding::Fonts,
    budget: &mut B,
) -> Result<Vec<AdmittedProductionFontV3>, ProductionResourceLedgerErrorV3<B::Error>> {
    use ProductionResourceLedgerStageV3 as Stage;
    if !B::OBSERVED {
        let mut fonts = BTreeMap::new();
        for (id, font) in native {
            if font.media_kind() == AdmittedFontMediaKind::SfntCff1
                || font.cff1_admission().is_some()
            {
                return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
            }
            fonts.insert(id, AdmittedProductionFontV3::TrueType(font));
        }
        for (id, font) in cff {
            if fonts
                .insert(id, AdmittedProductionFontV3::Cff1V2(font))
                .is_some()
            {
                return Err(ResourceAdmissionError::ConflictingLogicalResource.into());
            }
        }
        if fonts
            .values()
            .enumerate()
            .any(|(i, f)| f.font_face_id().get() as usize != i)
        {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        return Ok(fonts.into_values().collect());
    }
    for (id, font) in &native {
        work(budget, point(Stage::FontKinds, Some(*id), None), 64)?;
        if font.media_kind() == AdmittedFontMediaKind::SfntCff1 || font.cff1_admission().is_some() {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
    }
    // Preserve the ordinary error order: all kind checks, then conflicts, then
    // dense IDs. The borrowed scans allocate no second map.
    let mut a = native.keys().peekable();
    let mut z = cff.keys().peekable();
    while let (Some(x), Some(y)) = (a.peek(), z.peek()) {
        work(budget, point(Stage::FontConflicts, None, None), 64)?;
        match (*x).cmp(y) {
            std::cmp::Ordering::Equal => {
                return Err(ResourceAdmissionError::ConflictingLogicalResource.into())
            }
            std::cmp::Ordering::Less => {
                a.next();
            }
            std::cmp::Ordering::Greater => {
                z.next();
            }
        }
    }
    drop(z);
    let count = native
        .len()
        .checked_add(cff.len())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    let bytes = count
        .checked_mul(std::mem::size_of::<AdmittedProductionFontV3>())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    reserve(
        budget,
        ProductionResourceLedgerStorageV3::Fonts,
        count,
        bytes,
        point(Stage::Fonts, None, None),
    )?;
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    let mut a = native.into_iter().peekable();
    let mut z = cff.into_iter().peekable();
    while a.peek().is_some() || z.peek().is_some() {
        let take_native = match (a.peek(), z.peek()) {
            (Some((x, _)), Some((y, _))) => x < y,
            (Some(_), None) => true,
            _ => false,
        };
        let id = if take_native {
            a.peek().unwrap().0
        } else {
            z.peek().unwrap().0
        };
        work(budget, point(Stage::Fonts, Some(id), None), 64)?;
        let font = if take_native {
            AdmittedProductionFontV3::TrueType(a.next().unwrap().1)
        } else {
            AdmittedProductionFontV3::Cff1V2(z.next().unwrap().1)
        };
        if font.font_face_id().get() as usize != out.len() {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        out.push(font);
    }
    Ok(out)
}
pub(super) fn images<B: ProductionResourceLedgerBudgetV3>(
    images: crate::image_bindings::Images,
    budget: &mut B,
) -> Result<Vec<AdmittedImage>, ProductionResourceLedgerErrorV3<B::Error>> {
    if !B::OBSERVED {
        return Ok(images.into_values().collect());
    }
    let bytes = images
        .len()
        .checked_mul(std::mem::size_of::<AdmittedImage>())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    reserve(
        budget,
        ProductionResourceLedgerStorageV3::Images,
        images.len(),
        bytes,
        point(ProductionResourceLedgerStageV3::Images, None, None),
    )?;
    let mut out = Vec::new();
    out.try_reserve_exact(images.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for (id, image) in images {
        work(
            budget,
            point(ProductionResourceLedgerStageV3::Images, None, Some(id)),
            64,
        )?;
        out.push(image);
    }
    Ok(out)
}
pub(super) fn digest<B: ProductionResourceLedgerBudgetV3>(
    bytes: &[u8],
    budget: &mut B,
) -> Result<[u8; 32], ProductionResourceLedgerErrorV3<B::Error>> {
    if !B::OBSERVED {
        return Ok(sha256(bytes));
    }
    let mut hash = typaxis_core::Sha256::new();
    for chunk in bytes.chunks(256) {
        work(
            budget,
            point(ProductionResourceLedgerStageV3::CanonicalHash, None, None),
            chunk.len() as u64 + 64,
        )?;
        hash.update(chunk);
    }
    work(
        budget,
        point(ProductionResourceLedgerStageV3::CanonicalFinish, None, None),
        128,
    )?;
    Ok(hash.finish())
}
