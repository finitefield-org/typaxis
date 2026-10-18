//! Borrowed clip-closure scratch. Every loop, comparison, and swap is fallible.
use super::{permit, SafeSvgSourceBudget, SafeSvgSourceError, SafeSvgSourceStage as Stage};
use crate::ResourceAdmissionError;
use std::cmp::Ordering;
use typaxis_core::ImageResourceId;

struct Row<'a> {
    id: &'a str,
    segments: u64,
    used: bool,
}
pub(crate) struct Index<'a> {
    rows: Vec<Row<'a>>,
}
fn work<B: SafeSvgSourceBudget>(
    budget: &mut B,
    image: ImageResourceId,
    stage: Stage,
    amount: u64,
) -> Result<(), SafeSvgSourceError<B::Error>> {
    permit(budget, image, stage, 0, 0, amount)
}
fn compare<B: SafeSvgSourceBudget>(
    a: &str,
    z: &str,
    budget: &mut B,
    image: ImageResourceId,
    stage: Stage,
) -> Result<Ordering, SafeSvgSourceError<B::Error>> {
    // Both strings are borrowed from the immutable source. This upper-bounds
    // the bytes examined even for equal IDs or a long common prefix.
    let bytes = u64::try_from(a.len().min(z.len()))
        .ok()
        .and_then(|v| v.checked_mul(2))
        .and_then(|v| v.checked_add(64))
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    work(budget, image, stage, bytes)?;
    Ok(a.cmp(z))
}
impl<'a> Index<'a> {
    pub(crate) fn new<B: SafeSvgSourceBudget>(
        definitions: impl ExactSizeIterator<Item = (&'a str, u64)>,
        budget: &mut B,
        image: ImageResourceId,
    ) -> Result<Self, SafeSvgSourceError<B::Error>> {
        let count = definitions.len();
        let bytes = count
            .checked_mul(std::mem::size_of::<Row<'_>>())
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        permit(budget, image, Stage::ClipStorage, count, bytes, 64)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(count)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for (id, segments) in definitions {
            work(budget, image, Stage::ClipCollect, 64)?;
            if rows.len() == count {
                return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
            }
            rows.push(Row {
                id,
                segments,
                used: false,
            });
        }
        if rows.len() != count {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        let mut index = Self { rows };
        for root in (0..index.rows.len() / 2).rev() {
            index.sift(root, index.rows.len(), budget, image)?;
        }
        for end in (1..index.rows.len()).rev() {
            index.swap(0, end, budget, image)?;
            index.sift(0, end, budget, image)?;
        }
        for pair in index.rows.windows(2) {
            if compare(pair[0].id, pair[1].id, budget, image, Stage::ClipCompare)?.is_eq() {
                return Err(conflict());
            }
        }
        Ok(index)
    }
    fn swap<B: SafeSvgSourceBudget>(
        &mut self,
        a: usize,
        z: usize,
        budget: &mut B,
        image: ImageResourceId,
    ) -> Result<(), SafeSvgSourceError<B::Error>> {
        work(budget, image, Stage::ClipOrder, 128)?;
        self.rows.swap(a, z);
        Ok(())
    }
    fn sift<B: SafeSvgSourceBudget>(
        &mut self,
        mut root: usize,
        end: usize,
        budget: &mut B,
        image: ImageResourceId,
    ) -> Result<(), SafeSvgSourceError<B::Error>> {
        loop {
            work(budget, image, Stage::ClipOrder, 64)?;
            let child = root.checked_mul(2).and_then(|v| v.checked_add(1));
            let Some(mut child) = child.filter(|v| *v < end) else {
                return Ok(());
            };
            if child + 1 < end
                && compare(
                    self.rows[child].id,
                    self.rows[child + 1].id,
                    budget,
                    image,
                    Stage::ClipOrder,
                )?
                .is_lt()
            {
                child += 1;
            }
            if !compare(
                self.rows[root].id,
                self.rows[child].id,
                budget,
                image,
                Stage::ClipOrder,
            )?
            .is_lt()
            {
                return Ok(());
            }
            self.swap(root, child, budget, image)?;
            root = child;
        }
    }
    pub(crate) fn lookup<B: SafeSvgSourceBudget>(
        &mut self,
        id: &str,
        budget: &mut B,
        image: ImageResourceId,
    ) -> Result<Option<u64>, SafeSvgSourceError<B::Error>> {
        let (mut first, mut end) = (0, self.rows.len());
        while first < end {
            let middle = first + (end - first) / 2;
            match compare(self.rows[middle].id, id, budget, image, Stage::ClipLookup)? {
                Ordering::Less => first = middle + 1,
                Ordering::Greater => end = middle,
                Ordering::Equal => {
                    self.rows[middle].used = true;
                    return Ok(Some(self.rows[middle].segments));
                }
            }
        }
        Ok(None)
    }
    pub(crate) fn finish<B: SafeSvgSourceBudget>(
        self,
        budget: &mut B,
        image: ImageResourceId,
    ) -> Result<(), SafeSvgSourceError<B::Error>> {
        work(budget, image, Stage::ClipFinish, 64)?;
        for row in self.rows {
            work(budget, image, Stage::ClipFinish, 64)?;
            if !row.used {
                return Err(conflict());
            }
        }
        Ok(())
    }
}
fn conflict<E>() -> SafeSvgSourceError<E> {
    ResourceAdmissionError::InvalidSafeVectorV2(crate::SafeVectorFailureReason::ForbiddenFeature)
        .into()
}

#[cfg(test)]
#[path = "safe_svg_clip_closure_tests.rs"]
mod tests;
