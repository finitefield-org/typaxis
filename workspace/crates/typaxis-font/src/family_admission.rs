//! Borrowed names become a fallible, sorted family index on caller-owned cells.
use super::{FontFaceId, FontFamilyError, FontFamilyIndex, FontFamilyTable};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamilyStorage {
    Entries,
    NameBytes,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamilyAdmissionStage {
    Validate,
    Lookup,
    Entries,
    NameCopy,
    Insert,
}
pub trait FontFamilyAdmissionBudget {
    type Error;
    fn reserve(
        &mut self,
        site: FontFamilyStorage,
        face: Option<FontFaceId>,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error>;
    fn work(
        &mut self,
        stage: FontFamilyAdmissionStage,
        face: FontFaceId,
        amount: u64,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum FontFamilyAdmissionError<E> {
    Family(FontFamilyError),
    Capacity,
    ArithmeticOverflow,
    IteratorLength,
    Refused {
        error: E,
        face: Option<FontFaceId>,
        stage: FontFamilyAdmissionStage,
    },
}
fn work<B: FontFamilyAdmissionBudget>(
    budget: &mut B,
    stage: FontFamilyAdmissionStage,
    face: FontFaceId,
    n: u64,
) -> Result<(), FontFamilyAdmissionError<B::Error>> {
    budget
        .work(stage, face, n)
        .map_err(|error| FontFamilyAdmissionError::Refused {
            error,
            face: Some(face),
            stage,
        })
}
fn reserve<B: FontFamilyAdmissionBudget>(
    budget: &mut B,
    site: FontFamilyStorage,
    face: Option<FontFaceId>,
    rows: usize,
    bytes: usize,
) -> Result<(), FontFamilyAdmissionError<B::Error>> {
    let stage = match site {
        FontFamilyStorage::Entries => FontFamilyAdmissionStage::Entries,
        FontFamilyStorage::NameBytes => FontFamilyAdmissionStage::NameCopy,
    };
    budget
        .reserve(site, face, rows, bytes)
        .map_err(|error| FontFamilyAdmissionError::Refused { error, face, stage })
}
impl FontFamilyTable {
    /// Build the same family mapping without an owned input list or B-tree.
    /// Entry and name requests are logical bytes; allocator rounding and caller
    /// iterator work are separate. Every native index operation is prepaid.
    pub fn try_from_borrowed_with_budget<'a, B: FontFamilyAdmissionBudget>(
        families: impl ExactSizeIterator<Item = (&'a str, FontFaceId)>,
        budget: &mut B,
    ) -> Result<Self, FontFamilyAdmissionError<B::Error>> {
        use FontFamilyAdmissionError as Error;
        use FontFamilyAdmissionStage as Stage;
        let mut entries: Vec<(String, FontFaceId)> = Vec::new();
        let count = families.len();
        let slot = std::mem::size_of::<(String, FontFaceId)>();
        let bytes = count.checked_mul(slot).ok_or(Error::ArithmeticOverflow)?;
        if count > 0 {
            reserve(budget, FontFamilyStorage::Entries, None, count, bytes)?;
            entries
                .try_reserve_exact(count)
                .map_err(|_| Error::Capacity)?;
        }
        for (name, face) in families {
            if entries.len() >= count {
                return Err(Error::IteratorLength);
            }
            let length = u64::try_from(name.len()).map_err(|_| Error::ArithmeticOverflow)?;
            let validate = length
                .checked_mul(2)
                .and_then(|n| n.checked_add(64))
                .ok_or(Error::ArithmeticOverflow)?;
            work(budget, Stage::Validate, face, validate)?;
            if name.trim().is_empty() || name.chars().any(char::is_control) {
                return Err(Error::Family(FontFamilyError::EmptyFamily));
            }
            let lookup = super::family_work::lookup_allowance(entries.len(), name.len())
                .ok_or(Error::ArithmeticOverflow)?;
            work(budget, Stage::Lookup, face, lookup)?;
            let index =
                match entries.binary_search_by(|(candidate, _)| candidate.as_str().cmp(name)) {
                    Ok(_) => return Err(Error::Family(FontFamilyError::DuplicateFamily)),
                    Err(index) => index,
                };
            reserve(
                budget,
                FontFamilyStorage::NameBytes,
                Some(face),
                1,
                name.len(),
            )?;
            work(
                budget,
                Stage::NameCopy,
                face,
                length.checked_add(64).ok_or(Error::ArithmeticOverflow)?,
            )?;
            let mut owned = String::new();
            owned
                .try_reserve_exact(name.len())
                .map_err(|_| Error::Capacity)?;
            owned.push_str(name);
            let movement = (entries.len() - index)
                .checked_mul(slot)
                .and_then(|n| n.checked_add(64))
                .ok_or(Error::ArithmeticOverflow)?;
            work(
                budget,
                Stage::Insert,
                face,
                u64::try_from(movement).map_err(|_| Error::ArithmeticOverflow)?,
            )?;
            entries.insert(index, (owned, face));
        }
        if entries.len() != count {
            return Err(Error::IteratorLength);
        }
        Ok(Self {
            by_family: FontFamilyIndex::Sorted(entries),
        })
    }
}

#[cfg(test)]
#[path = "family_admission_tests.rs"]
mod tests;
