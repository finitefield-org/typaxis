// Copyright 2026 Typaxis contributors.
// Licensed under MIT OR Apache-2.0, as the surrounding Unicode-bidi code.
//! Fallible scratch requests for the private Typaxis integration.
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Site {
    OriginalClasses,
    Paragraphs,
    ParagraphFlags,
    IsolateStack,
    Levels,
    ProcessingClasses,
    ExplicitStack,
    LevelRuns,
    IsolatingSequences,
    SequenceRuns,
    SequenceStack,
    Sequences,
    WeakEt,
    WeakBn,
    NeutralRun,
    BracketStack,
    BracketPairs,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Refused,
    AllocationFailure,
    ArithmeticOverflow,
}

/// The callback runs before every requested allocation. It grants no authority
/// over text or levels and is not retained in the returned BidiInfo.
pub trait Budget {
    const OBSERVED: bool = true;
    fn reserve(&mut self, site: Site, count: usize, element_bytes: usize) -> Result<(), Error>;
    fn work(&mut self, amount: usize) -> Result<(), Error>;
}
pub(crate) struct NoScratch;
impl Budget for NoScratch {
    const OBSERVED: bool = false;
    fn reserve(&mut self, _: Site, _: usize, _: usize) -> Result<(), Error> {
        Ok(())
    }
    fn work(&mut self, _: usize) -> Result<(), Error> {
        Ok(())
    }
}
#[inline(always)]
pub(crate) fn work<B: Budget>(budget: &mut B, count: usize) -> Result<(), Error> {
    if B::OBSERVED {
        budget.work(count)
    } else {
        Ok(())
    }
}
pub(crate) fn with_capacity<T, B: Budget>(
    count: usize,
    site: Site,
    budget: &mut B,
) -> Result<Vec<T>, Error> {
    if !B::OBSERVED {
        return Ok(Vec::with_capacity(count));
    }
    let mut values = Vec::new();
    reserve_exact(&mut values, count, site, budget)?;
    Ok(values)
}
pub(crate) fn reserve_exact<T, B: Budget>(
    values: &mut Vec<T>,
    additional: usize,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if !B::OBSERVED {
        values.reserve_exact(additional);
        return Ok(());
    }
    let target = values
        .len()
        .checked_add(additional)
        .ok_or(Error::ArithmeticOverflow)?;
    if target > values.capacity() {
        budget.reserve(site, target, core::mem::size_of::<T>())?;
        values
            .try_reserve_exact(additional)
            .map_err(|_| Error::AllocationFailure)?;
    }
    Ok(())
}
pub(crate) fn push<T, B: Budget>(
    values: &mut Vec<T>,
    value: T,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if !B::OBSERVED {
        values.push(value);
        return Ok(());
    }
    work(budget, 1)?;
    if values.len() == values.capacity() {
        let target = values
            .capacity()
            .checked_mul(2)
            .ok_or(Error::ArithmeticOverflow)?
            .max(4)
            .max(
                values
                    .len()
                    .checked_add(1)
                    .ok_or(Error::ArithmeticOverflow)?,
            );
        reserve_exact(values, target - values.len(), site, budget)?;
    }
    values.push(value);
    Ok(())
}
pub(crate) fn extend<T, B: Budget>(
    values: &mut Vec<T>,
    items: impl IntoIterator<Item = T>,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if !B::OBSERVED {
        values.extend(items);
        return Ok(());
    }
    for value in items {
        push(values, value, site, budget)?;
    }
    Ok(())
}
pub(crate) fn single<T, B: Budget>(value: T, site: Site, budget: &mut B) -> Result<Vec<T>, Error> {
    if !B::OBSERVED {
        return Ok(alloc::vec![value]);
    }
    let mut v = with_capacity(1, site, budget)?;
    work(budget, 1)?;
    v.push(value);
    Ok(v)
}
pub(crate) fn clone_slice<T: Clone, B: Budget>(
    values: &[T],
    site: Site,
    budget: &mut B,
) -> Result<Vec<T>, Error> {
    if !B::OBSERVED {
        return Ok(values.to_vec());
    }
    let mut v = with_capacity(values.len(), site, budget)?;
    for value in values {
        work(budget, 1)?;
        v.push(value.clone());
    }
    Ok(v)
}
pub(crate) fn resize<T: Clone, B: Budget>(
    values: &mut Vec<T>,
    target: usize,
    value: T,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if !B::OBSERVED {
        values.resize(target, value);
        return Ok(());
    }
    if target > values.len() {
        reserve_exact(values, target - values.len(), site, budget)?;
        work(budget, target - values.len())?;
    }
    values.resize(target, value);
    Ok(())
}
pub(crate) fn sort_by_key<T, K: Ord, B: Budget>(
    values: &mut [T],
    key: impl Fn(&T) -> K,
    budget: &mut B,
) -> Result<(), Error> {
    if !B::OBSERVED {
        values.sort_by_key(key);
        return Ok(());
    }
    fn sift<T, K: Ord, B: Budget>(
        v: &mut [T],
        mut root: usize,
        end: usize,
        key: &impl Fn(&T) -> K,
        b: &mut B,
    ) -> Result<(), Error> {
        while let Some(left) = root
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n < end)
        {
            let mut child = left;
            if left + 1 < end {
                work(b, 1)?;
                if key(&v[left]) < key(&v[left + 1]) {
                    child += 1;
                }
            }
            work(b, 1)?;
            if key(&v[root]) >= key(&v[child]) {
                break;
            }
            work(b, 1)?;
            v.swap(root, child);
            root = child;
        }
        Ok(())
    }
    for root in (0..values.len() / 2).rev() {
        sift(values, root, values.len(), &key, budget)?;
    }
    for end in (1..values.len()).rev() {
        work(budget, 1)?;
        values.swap(0, end);
        sift(values, 0, end, &key, budget)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "scratch_tests.rs"]
mod tests;
