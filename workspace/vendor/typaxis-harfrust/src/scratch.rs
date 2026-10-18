//! Fallible requests for private Typaxis cache, input, plan and glyph storage.
use alloc::vec::Vec;

/// An allocation performed by an observed cache, input, plan or glyph operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Site {
    /// GSUB lookup descriptors.
    GsubLookups,
    /// GSUB subtable descriptors.
    GsubSubtables,
    /// GPOS lookup descriptors.
    GposLookups,
    /// GPOS subtable descriptors.
    GposSubtables,
    /// GDEF mark-set coverage digests.
    GdefMarkSets,
    /// Initial or appended Unicode input glyph information.
    UnicodeInfos,
    /// Position slots paired with the Unicode input information.
    UnicodePositions,
    /// Owned ASCII-folded input language tag.
    LanguageString,
    /// Feature declarations while collecting a shape plan.
    PlanFeatureInfos,
    /// GSUB pause declarations while collecting a shape plan.
    PlanGsubPauses,
    /// GPOS pause declarations while collecting a shape plan.
    PlanGposPauses,
    /// Compiled shape-plan feature maps.
    PlanFeatures,
    /// Compiled shape-plan GSUB lookup maps.
    PlanGsubLookups,
    /// Compiled shape-plan GPOS lookup maps.
    PlanGposLookups,
    /// Compiled shape-plan GSUB stages.
    PlanGsubStages,
    /// Compiled shape-plan GPOS stages.
    PlanGposStages,
    /// The single owned script-specific shape-plan record.
    PlanData,
    /// Glyph-information storage grown while running the original shaper.
    GlyphInfos,
    /// Position storage paired with information grown while shaping.
    GlyphPositions,
    /// Requested AAT feature ranges.
    AatFeatures,
    /// AAT feature start/end events.
    AatFeatureEvents,
    /// Active AAT feature records while sweeping events.
    AatActiveFeatures,
    /// Current AAT feature snapshot for a compiled range.
    AatCurrentFeatures,
    /// Outer AAT chain vector.
    AatChainFlags,
    /// Per-chain AAT range flags.
    AatRangeFlags,
    /// Temporary values with original indices for a stable AAT sort.
    AatSortRecords,
    /// Spilled OpenType contextual and ligature match positions.
    MatchPositions,
    /// Indexed glyph records for the observed stable Indic position sort.
    IndicSortRecords,
}

/// A failed scratch request. Malformed font tables retain the original skip policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The caller refused a charge.
    Refused,
    /// The requested allocation failed after payment.
    AllocationFailure,
    /// A requested size could not be represented.
    ArithmeticOverflow,
    /// Input or shaped glyph count exceeds a buffer length or cluster-index limit.
    InputLimit,
    /// The input language is empty, as rejected by the original parser.
    InvalidLanguage,
    /// An internal shaper has no fallible plan-data constructor.
    UnsupportedPlanData,
}

/// Requests are paid before allocation or the corresponding instrumented work.
/// No constructor, input operation or shaping result retains this borrow.
pub trait Budget {
    /// False preserves the original infallible allocation path.
    const OBSERVED: bool = true;
    /// Pay a complete requested target capacity, including every reallocation.
    fn reserve(&mut self, site: Site, count: usize, element_bytes: usize) -> Result<(), Error>;
    /// Pay instrumented traversal, initialization or input-writing work.
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

pub(crate) fn reserve<T, B: Budget>(
    values: &mut Vec<T>,
    target: usize,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if target > values.capacity() {
        let additional = target
            .checked_sub(values.len())
            .ok_or(Error::ArithmeticOverflow)?;
        if B::OBSERVED {
            target
                .checked_mul(core::mem::size_of::<T>())
                .ok_or(Error::ArithmeticOverflow)?;
            budget.reserve(site, target, core::mem::size_of::<T>())?;
            values
                .try_reserve_exact(additional)
                .map_err(|_| Error::AllocationFailure)?;
        } else {
            values.reserve_exact(additional);
        }
    }
    Ok(())
}

pub(crate) fn resize<T: Clone, B: Budget>(
    values: &mut Vec<T>,
    target: usize,
    value: T,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if B::OBSERVED && target > values.len() {
        reserve(values, target, site, budget)?;
        work(budget, target - values.len())?;
    }
    values.resize(target, value);
    Ok(())
}

pub(crate) fn push<T, B: Budget>(
    values: &mut Vec<T>,
    value: T,
    site: Site,
    budget: &mut B,
) -> Result<(), Error> {
    if B::OBSERVED {
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
            reserve(values, target, site, budget)?;
        }
    }
    values.push(value);
    Ok(())
}

// The original shaper callbacks use a concrete planner type. Erase only the
// temporary budget dispatch, without storing it in a compiled map or plan.
pub(crate) trait PlanObserver {
    fn reserve(&mut self, site: Site, count: usize, bytes: usize) -> Result<(), Error>;
    fn work(&mut self, count: usize) -> Result<(), Error>;
}

pub(crate) struct ObservePlan<'a, B: Budget>(pub &'a mut B);
impl<B: Budget> PlanObserver for ObservePlan<'_, B> {
    fn reserve(&mut self, site: Site, count: usize, bytes: usize) -> Result<(), Error> {
        self.0.reserve(site, count, bytes)
    }
    fn work(&mut self, count: usize) -> Result<(), Error> {
        self.0.work(count)
    }
}

struct ErasedPlan<'a>(&'a mut dyn PlanObserver);
impl Budget for ErasedPlan<'_> {
    fn reserve(&mut self, site: Site, count: usize, bytes: usize) -> Result<(), Error> {
        self.0.reserve(site, count, bytes)
    }
    fn work(&mut self, count: usize) -> Result<(), Error> {
        self.0.work(count)
    }
}

pub(crate) struct PlanScratch<'a> {
    observer: Option<&'a mut dyn PlanObserver>,
    error: Option<Error>,
}

impl<'a> PlanScratch<'a> {
    pub(crate) fn new(observer: Option<&'a mut dyn PlanObserver>) -> Self {
        Self {
            observer,
            error: None,
        }
    }

    pub(crate) fn observed(&self) -> bool {
        self.observer.is_some()
    }

    pub(crate) fn status(&self) -> Result<(), Error> {
        self.error.map_or(Ok(()), Err)
    }

    pub(crate) fn fail(&mut self, error: Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    pub(crate) fn reserve<T>(
        &mut self,
        values: &mut Vec<T>,
        target: usize,
        site: Site,
    ) -> Result<(), Error> {
        self.status()?;
        let result = match self.observer.as_mut() {
            Some(observer) => reserve(values, target, site, &mut ErasedPlan(*observer)),
            None => {
                values.reserve_exact(target.saturating_sub(values.len()));
                Ok(())
            }
        };
        self.retain(result)
    }

    fn retain<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        if let Err(error) = result {
            self.error = Some(error);
        }
        result
    }

    pub(crate) fn work(&mut self, amount: usize) -> Result<(), Error> {
        self.status()?;
        let result = match self.observer.as_mut() {
            Some(observer) => observer.work(amount),
            None => Ok(()),
        };
        self.retain(result)
    }

    pub(crate) fn resize_match_positions(
        &mut self,
        values: &mut smallvec::SmallVec<[usize; 4]>,
        target: usize,
    ) -> Result<(), Error> {
        self.status()?;
        let result = match self.observer.as_mut() {
            Some(observer) => (|| {
                if target > values.capacity() {
                    // SmallVec::resize uses reserve, whose native growth target
                    // is the next power of two. Keep that target, including for
                    // the first spill from its four inline slots.
                    let capacity = target
                        .checked_next_power_of_two()
                        .ok_or(Error::ArithmeticOverflow)?;
                    core::alloc::Layout::array::<usize>(capacity)
                        .map_err(|_| Error::ArithmeticOverflow)?;
                    observer.reserve(
                        Site::MatchPositions,
                        capacity,
                        core::mem::size_of::<usize>(),
                    )?;
                    observer.work(values.len())?;
                    values.try_reserve_exact(capacity - values.len()).map_err(
                        |error| match error {
                            smallvec::CollectionAllocErr::CapacityOverflow => {
                                Error::ArithmeticOverflow
                            }
                            smallvec::CollectionAllocErr::AllocErr { .. } => {
                                Error::AllocationFailure
                            }
                        },
                    )?;
                }
                if target > values.len() {
                    observer.work(target - values.len())?;
                }
                values.resize(target, 0);
                Ok(())
            })(),
            None => {
                values.resize(target, 0);
                Ok(())
            }
        };
        self.retain(result)
    }

    pub(crate) fn push<T>(
        &mut self,
        values: &mut Vec<T>,
        value: T,
        site: Site,
    ) -> Result<(), Error> {
        self.status()?;
        let result = match self.observer.as_mut() {
            Some(observer) => push(values, value, site, &mut ErasedPlan(*observer)),
            None => {
                values.push(value);
                Ok(())
            }
        };
        self.retain(result)
    }

    pub(crate) fn language(&mut self, text: &str) -> Result<crate::Language, Error> {
        self.status()?;
        let result = match self.observer.as_mut() {
            Some(observer) => {
                crate::Language::try_from_str_with_scratch(text, &mut ErasedPlan(*observer))
            }
            None => text.parse().map_err(|_| Error::InvalidLanguage),
        };
        self.retain(result)
    }

    pub(crate) fn single<T>(&mut self, create: impl FnOnce() -> T) -> Result<Vec<T>, Error> {
        self.status()?;
        let mut values = Vec::new();
        if let Some(observer) = self.observer.as_mut() {
            let result = reserve(&mut values, 1, Site::PlanData, &mut ErasedPlan(*observer));
            self.retain(result)?;
        }
        self.work(1)?;
        values.push(create());
        Ok(values)
    }

    pub(crate) fn sort<T: Ord>(&mut self, values: &mut [T]) -> Result<(), Error> {
        self.status()?;
        if !self.observed() {
            values.sort();
            return Ok(());
        }
        // The feature sequence key is unique; feature-map tags are unique;
        // lookup Ord compares every retained field. Equal keys therefore need
        // no distinct stable ordering. Heapsort needs no hidden sort allocation
        // and can stop before every unpaid comparison or swap.
        fn sift<T: Ord>(
            values: &mut [T],
            mut root: usize,
            end: usize,
            scratch: &mut PlanScratch<'_>,
        ) -> Result<(), Error> {
            loop {
                let left = root
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(Error::ArithmeticOverflow)?;
                if left >= end {
                    return Ok(());
                }
                let mut child = left;
                if left + 1 < end {
                    scratch.work(1)?;
                    if values[left] < values[left + 1] {
                        child += 1;
                    }
                }
                scratch.work(1)?;
                if values[root] >= values[child] {
                    return Ok(());
                }
                scratch.work(3)?;
                values.swap(root, child);
                root = child;
            }
        }
        for root in (0..values.len() / 2).rev() {
            sift(values, root, values.len(), self)?;
        }
        for end in (1..values.len()).rev() {
            self.work(3)?;
            values.swap(0, end);
            sift(values, 0, end, self)?;
        }
        Ok(())
    }

    pub(crate) fn stable_sort<T: Copy + Ord>(&mut self, values: &mut [T]) -> Result<(), Error> {
        self.status()?;
        if !self.observed() {
            values.sort();
            return Ok(());
        }
        if values.len() < 2 {
            return Ok(());
        }
        // Equal AAT keys can carry different features. The original index
        // makes their order explicit, so the paid heap sort preserves ties.
        // Keep the caller's values intact until all fallible steps succeed.
        let mut records = Vec::new();
        self.reserve(&mut records, values.len(), Site::AatSortRecords)?;
        self.work(values.len())?;
        for (index, value) in values.iter().copied().enumerate() {
            records.push((value, index));
        }
        self.sort(&mut records)?;
        self.work(values.len())?;
        for (value, (sorted, _)) in values.iter_mut().zip(records) {
            *value = sorted;
        }
        Ok(())
    }

    pub(crate) fn stable_sort_by_key<T: Copy, K: Ord>(
        &mut self,
        values: &mut [T],
        mut key: impl FnMut(&T) -> K,
        site: Site,
    ) -> Result<(), Error> {
        self.status()?;
        if !self.observed() {
            values.sort_by_key(key);
            return Ok(());
        }
        if values.len() < 2 {
            return Ok(());
        }
        // Compare only the key and original index: glyph records themselves
        // have no meaningful Ord. The unique index preserves all stable ties.
        struct Record<Key, Value> {
            key: Key,
            index: usize,
            value: Value,
        }
        impl<Key: Ord, Value> PartialEq for Record<Key, Value> {
            fn eq(&self, other: &Self) -> bool {
                self.key == other.key && self.index == other.index
            }
        }
        impl<Key: Ord, Value> Eq for Record<Key, Value> {}
        impl<Key: Ord, Value> PartialOrd for Record<Key, Value> {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl<Key: Ord, Value> Ord for Record<Key, Value> {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.key
                    .cmp(&other.key)
                    .then_with(|| self.index.cmp(&other.index))
            }
        }
        let mut records = Vec::new();
        self.reserve(&mut records, values.len(), site)?;
        self.work(values.len())?;
        for (index, value) in values.iter().copied().enumerate() {
            records.push(Record {
                key: key(&value),
                index,
                value,
            });
        }
        self.sort(&mut records)?;
        // Refusal during reservation, keying or sorting leaves the original
        // values intact. Commit only after paying every final glyph copy.
        self.work(values.len())?;
        for (value, record) in values.iter_mut().zip(records) {
            *value = record.value;
        }
        Ok(())
    }
}
