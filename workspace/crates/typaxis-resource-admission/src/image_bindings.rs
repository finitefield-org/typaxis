//! Native map semantics with an opt-in, fallibly reserved sorted image table.
use crate::safe_svg_source::{permit, reserve_vec};
use crate::{
    AdmittedImage, ImageResourceId, ResourceAdmissionError, SafeSvgSourceBudget,
    SafeSvgSourceError, SafeSvgSourceStage as S,
};
use std::collections::{btree_map, BTreeMap};

#[derive(Debug)]
pub(crate) enum Images {
    Native(BTreeMap<ImageResourceId, AdmittedImage>),
    Reserved {
        values: Vec<AdmittedImage>,
        bound: usize,
    },
}
impl Images {
    pub(crate) fn new() -> Self {
        Self::Native(BTreeMap::new())
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Native(v) => v.len(),
            Self::Reserved { values, .. } => values.len(),
        }
    }
    pub(crate) fn contains_key(&self, id: &ImageResourceId) -> bool {
        match self {
            Self::Native(v) => v.contains_key(id),
            Self::Reserved { values, .. } => {
                values.binary_search_by_key(id, |v| v.image_id()).is_ok()
            }
        }
    }
    pub(crate) fn values(&self) -> Values<'_> {
        match self {
            Self::Native(v) => Values::Native(v.values()),
            Self::Reserved { values, .. } => Values::Reserved(values.iter()),
        }
    }
    pub(crate) fn into_values(self) -> OwnedValues {
        match self {
            Self::Native(v) => OwnedValues::Native(v.into_values()),
            Self::Reserved { values, .. } => OwnedValues::Reserved(values.into_iter()),
        }
    }
    pub(crate) fn insert(
        &mut self,
        id: ImageResourceId,
        image: AdmittedImage,
    ) -> Result<Option<AdmittedImage>, ResourceAdmissionError> {
        if image.image_id() != id {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
        match self {
            Self::Native(v) => Ok(v.insert(id, image)),
            Self::Reserved { values, bound } => {
                match values.binary_search_by_key(&id, |v| v.image_id()) {
                    Ok(at) => Ok(Some(std::mem::replace(&mut values[at], image))),
                    Err(at) => {
                        if values.len() >= *bound {
                            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                        }
                        values.insert(at, image);
                        Ok(None)
                    }
                }
            }
        }
    }
    pub(crate) fn prepare<B: SafeSvgSourceBudget>(
        &mut self,
        bound: usize,
        budget: &mut B,
        image: ImageResourceId,
    ) -> Result<usize, SafeSvgSourceError<B::Error>> {
        if let Self::Native(old) = self {
            if old.len() > bound {
                return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
            }
            let mut values = reserve_vec(budget, image, S::BindingStorage, bound)?;
            // Refusal during preparation preserves the entire native map.
            for _ in 0..old.len() {
                permit(budget, image, S::BindingMigrate, 0, 0, 64)?;
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
        permit(budget, image, S::BindingLookup, 0, 0, 64)?;
        while first < end {
            permit(budget, image, S::BindingLookup, 0, 0, 64)?;
            let at = first + (end - first) / 2;
            match values[at].image_id().cmp(&image) {
                std::cmp::Ordering::Less => first = at + 1,
                std::cmp::Ordering::Greater => end = at,
                std::cmp::Ordering::Equal => {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource.into())
                }
            }
        }
        if values.len() >= bound {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        let shifted = (values.len() - first)
            .checked_mul(std::mem::size_of::<AdmittedImage>())
            .and_then(|n| u64::try_from(n).ok())
            .and_then(|n| n.checked_add(64))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        permit(budget, image, S::BindingInsert, 0, 0, shifted)?;
        Ok(first)
    }
    pub(crate) fn insert_prepared(
        &mut self,
        at: usize,
        image: AdmittedImage,
    ) -> Result<(), ResourceAdmissionError> {
        let Self::Reserved { values, bound } = self else {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        };
        if values.len() >= *bound
            || at > values.len()
            || at > 0 && values[at - 1].image_id() >= image.image_id()
            || at < values.len() && values[at].image_id() <= image.image_id()
        {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
        values.insert(at, image);
        Ok(())
    }
}
pub(crate) enum Values<'a> {
    Native(btree_map::Values<'a, ImageResourceId, AdmittedImage>),
    Reserved(std::slice::Iter<'a, AdmittedImage>),
}
impl<'a> Iterator for Values<'a> {
    type Item = &'a AdmittedImage;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Native(v) => v.next(),
            Self::Reserved(v) => v.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();
        (n, Some(n))
    }
}
impl ExactSizeIterator for Values<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Native(v) => v.len(),
            Self::Reserved(v) => v.len(),
        }
    }
}
pub(crate) enum OwnedValues {
    Native(btree_map::IntoValues<ImageResourceId, AdmittedImage>),
    Reserved(std::vec::IntoIter<AdmittedImage>),
}
impl Iterator for OwnedValues {
    type Item = AdmittedImage;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Native(v) => v.next(),
            Self::Reserved(v) => v.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();
        (n, Some(n))
    }
}
impl ExactSizeIterator for OwnedValues {
    fn len(&self) -> usize {
        match self {
            Self::Native(v) => v.len(),
            Self::Reserved(v) => v.len(),
        }
    }
}
pub(crate) struct Entries(OwnedValues);
impl Iterator for Entries {
    type Item = (ImageResourceId, AdmittedImage);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|v| (v.image_id(), v))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl ExactSizeIterator for Entries {}
impl IntoIterator for Images {
    type Item = (ImageResourceId, AdmittedImage);
    type IntoIter = Entries;
    fn into_iter(self) -> Entries {
        Entries(self.into_values())
    }
}

#[cfg(test)]
#[path = "image_bindings_tests.rs"]
mod tests;
