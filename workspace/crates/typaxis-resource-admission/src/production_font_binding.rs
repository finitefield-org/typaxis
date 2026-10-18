//! Fallible CFF binding storage. Native entry points retain their map policy.
use super::*;
use crate::staging_catalog_budget::{amount, charge, point, rows, string, uri};
use ResourceCatalogStage as S;

#[derive(Debug)]
pub enum ProductionFontBindingCauseV3<E> {
    Admission(typaxis_font::CffAdmissionBudgetFailureV2<E>),
    Binding(ResourceCatalogBudgetFailure<E>),
}
#[derive(Debug)]
pub struct ProductionFontBindingBudgetFailureV3<E> {
    pub font_face_id: FontFaceId,
    pub failure: ProductionFontBindingCauseV3<E>,
}
impl<E: std::fmt::Debug> std::fmt::Display for ProductionFontBindingBudgetFailureV3<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production font binding budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ProductionFontBindingBudgetFailureV3<E> {}

pub type ProductionFontBindingBudgetResultV3<T, E> =
    Result<Result<T, ProductionResourceErrorV3>, ProductionFontBindingBudgetFailureV3<E>>;

pub(super) type Permit<C, E> = fn(&mut C, ResourceCatalogRequest) -> Result<(), E>;
pub(super) fn permit<C: ResourceCatalogBudget>(
    budget: &mut C,
    request: ResourceCatalogRequest,
) -> Result<(), ProductionFontBindingCauseV3<C::Error>> {
    budget.charge(request).map_err(|error| {
        ProductionFontBindingCauseV3::Binding(ResourceCatalogBudgetFailure { error, request })
    })
}
pub(super) struct Budget<'a, C, E> {
    pub budget: &'a mut C,
    pub permit: Permit<C, E>,
    pub id: FontFaceId,
}
impl<C, E> ResourceCatalogBudget for Budget<'_, C, E> {
    type Error = E;
    fn charge(&mut self, mut request: ResourceCatalogRequest) -> Result<(), E> {
        request.font_face_id = Some(self.id);
        (self.permit)(self.budget, request)
    }
}

pub(super) fn checks<B: ResourceCatalogBudget>(
    declaration: &FontFaceDeclaration,
    source: &PendingResourceBytes,
    existing: usize,
    budget: &mut B,
) -> Result<(), ResourceCatalogError<B::Error>> {
    // URI comparison plus a conservative bound for both existing binding maps.
    let bytes = source
        .uri()
        .as_str()
        .len()
        .checked_add(declaration.uri.as_str().len())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    let work = amount(bytes, 2, 64)?
        .checked_add(amount(existing, 64, 64)?)
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    charge(
        budget,
        ResourceCatalogRequest {
            work,
            ..point(S::CffBindingChecks, Some(declaration.font_face_id), None)
        },
    )
}
pub(super) fn declaration<B: ResourceCatalogBudget>(
    value: &FontFaceDeclaration,
    budget: &mut B,
) -> Result<FontFaceDeclaration, ResourceCatalogError<B::Error>> {
    let id = Some(value.font_face_id);
    let family = string(&value.family, point(S::CffBindingFamily, id, None), budget)?;
    let uri = uri(&value.uri, point(S::CffBindingUri, id, None), budget)?;
    Ok(FontFaceDeclaration {
        font_face_id: value.font_face_id,
        family,
        uri,
        face_index: value.face_index,
        expected_sha256: value.expected_sha256,
    })
}

#[derive(Debug)]
pub(super) enum Fonts {
    Native(BTreeMap<FontFaceId, AdmittedCff1FontV2>),
    Reserved {
        values: Vec<AdmittedCff1FontV2>,
        bound: usize,
    },
}
impl Fonts {
    pub(super) fn new() -> Self {
        Self::Native(BTreeMap::new())
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Native(v) => v.len(),
            Self::Reserved { values, .. } => values.len(),
        }
    }
    pub(super) fn contains_key(&self, id: &FontFaceId) -> bool {
        match self {
            Self::Native(v) => v.contains_key(id),
            Self::Reserved { values, .. } => values
                .binary_search_by_key(id, |f| f.declaration.font_face_id)
                .is_ok(),
        }
    }
    pub(super) fn keys(&self) -> impl Iterator<Item = FontFaceId> + '_ {
        self.values().map(|f| f.declaration.font_face_id)
    }
    fn values(&self) -> Values<'_> {
        match self {
            Self::Native(v) => Values::Native(v.values()),
            Self::Reserved { values, .. } => Values::Reserved(values.iter()),
        }
    }
    pub(super) fn prepare<B: ResourceCatalogBudget>(
        &mut self,
        bound: usize,
        id: FontFaceId,
        budget: &mut B,
    ) -> Result<usize, ResourceCatalogError<B::Error>> {
        if let Self::Native(old) = self {
            if old.len() > bound {
                return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
            }
            let mut values = rows(bound, S::CffBindingStorage, budget)?;
            // Pay for every move before consuming any of the original map.
            for _ in 0..old.len() {
                charge(
                    budget,
                    ResourceCatalogRequest {
                        work: amount(std::mem::size_of::<AdmittedCff1FontV2>(), 1, 64)?,
                        ..point(S::CffBindingMigrate, Some(id), None)
                    },
                )?;
            }
            let Self::Native(old) = std::mem::replace(self, Self::new()) else {
                unreachable!()
            };
            values.extend(old.into_values());
            *self = Self::Reserved { values, bound };
        }
        let Self::Reserved {
            values,
            bound: reserved,
        } = self
        else {
            unreachable!()
        };
        if *reserved != bound {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        let (mut first, mut end) = (0, values.len());
        while first < end {
            charge(budget, point(S::CffBindingLookup, Some(id), None))?;
            let middle = first + (end - first) / 2;
            match values[middle].declaration.font_face_id.cmp(&id) {
                std::cmp::Ordering::Less => first = middle + 1,
                std::cmp::Ordering::Greater => end = middle,
                std::cmp::Ordering::Equal => {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource.into())
                }
            }
        }
        if values.len() >= bound {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        let moved = values
            .len()
            .checked_sub(first)
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(std::mem::size_of::<AdmittedCff1FontV2>()))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(
            budget,
            ResourceCatalogRequest {
                work: amount(moved, 1, 64)?,
                ..point(S::CffBindingInsert, Some(id), None)
            },
        )?;
        Ok(first)
    }
    pub(super) fn insert(
        &mut self,
        id: FontFaceId,
        font: AdmittedCff1FontV2,
        position: Option<usize>,
    ) -> Result<(), ResourceAdmissionError> {
        if font.declaration.font_face_id != id {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
        match self {
            Self::Native(v) => {
                v.insert(id, font);
            }
            Self::Reserved { values, bound } => {
                // The observer supplies its paid insertion position. Legacy
                // calls can use this retained storage without a new allocation.
                let at = match position {
                    Some(at) => at,
                    None => values
                        .binary_search_by_key(&id, |f| f.declaration.font_face_id)
                        .map_or_else(Ok, |_| {
                            Err(ResourceAdmissionError::ConflictingLogicalResource)
                        })?,
                };
                if at > values.len() {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if values.len() >= *bound || values.len() == values.capacity() {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                values.insert(at, font);
            }
        }
        Ok(())
    }
}
enum Values<'a> {
    Native(std::collections::btree_map::Values<'a, FontFaceId, AdmittedCff1FontV2>),
    Reserved(std::slice::Iter<'a, AdmittedCff1FontV2>),
}
impl<'a> Iterator for Values<'a> {
    type Item = &'a AdmittedCff1FontV2;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Native(v) => v.next(),
            Self::Reserved(v) => v.next(),
        }
    }
}
pub(super) enum Entries {
    Native(std::collections::btree_map::IntoIter<FontFaceId, AdmittedCff1FontV2>),
    Reserved(std::vec::IntoIter<AdmittedCff1FontV2>),
}
impl Iterator for Entries {
    type Item = (FontFaceId, AdmittedCff1FontV2);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Native(v) => v.next(),
            Self::Reserved(v) => v.next().map(|f| (f.declaration.font_face_id, f)),
        }
    }
}
impl IntoIterator for Fonts {
    type Item = (FontFaceId, AdmittedCff1FontV2);
    type IntoIter = Entries;
    fn into_iter(self) -> Entries {
        match self {
            Self::Native(v) => Entries::Native(v.into_iter()),
            Self::Reserved { values, .. } => Entries::Reserved(values.into_iter()),
        }
    }
}
