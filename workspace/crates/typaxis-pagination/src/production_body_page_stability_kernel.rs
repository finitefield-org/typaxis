//! Repeat actual selection/placement with one bounded record/work owner.
use super::*;
pub(super) trait StableSearch {
    type Sequence;
    fn maximum_passes(&self) -> u16;
    fn charge_pass(&mut self) -> Result<(), ProductionBodyPaginationError>;
    fn select(&mut self) -> Result<Self::Sequence, ProductionBodyPaginationError>;
    fn same(
        &mut self,
        left: &Self::Sequence,
        right: &Self::Sequence,
    ) -> Result<bool, ProductionBodyPaginationError>;
    fn records(&self) -> u64;
    fn work(&self) -> u64;
}
pub(super) struct StableProjection<S> {
    pub sequence: S,
    pub passes: u16,
    pub records: u64,
    pub work: u64,
}
pub(super) fn converge<S: StableSearch>(
    search: &mut S,
    remaining_passes: u16,
) -> Result<StableProjection<S::Sequence>, ProductionBodyPaginationError> {
    converge_counted(search, remaining_passes, &mut 0)
}

/// Count only accepted pass starts; selection/comparison failure keeps them.
pub(super) fn converge_counted<S: StableSearch>(
    search: &mut S,
    remaining_passes: u16,
    begun_passes: &mut u16,
) -> Result<StableProjection<S::Sequence>, ProductionBodyPaginationError> {
    *begun_passes = 0;
    let maximum = search.maximum_passes().min(remaining_passes);
    if maximum < 2 {
        return Err(error(NodeId::new(0), E::PagePassLimit));
    }
    search.charge_pass()?;
    *begun_passes += 1;
    let mut previous = search.select()?;
    for pass in 2..=maximum {
        search.charge_pass()?;
        *begun_passes += 1;
        let current = search.select()?;
        if search.same(&previous, &current)? {
            return Ok(StableProjection {
                sequence: current,
                passes: pass,
                records: search.records(),
                work: search.work(),
            });
        }
        previous = current;
    }
    Err(error(NodeId::new(0), E::PagePassLimit))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct ChangingPages {
        values: [u8; 3],
        passes: usize,
        comparisons: u64,
        maximum: u16,
    }
    impl StableSearch for ChangingPages {
        type Sequence = u8;
        fn maximum_passes(&self) -> u16 {
            self.maximum
        }
        fn charge_pass(&mut self) -> Result<(), ProductionBodyPaginationError> {
            self.passes += 1;
            Ok(())
        }
        fn select(&mut self) -> Result<u8, ProductionBodyPaginationError> {
            Ok(self.values[self.passes - 1])
        }
        fn same(&mut self, left: &u8, right: &u8) -> Result<bool, ProductionBodyPaginationError> {
            self.comparisons += 1;
            Ok(left == right)
        }
        fn records(&self) -> u64 {
            self.passes as u64
        }
        fn work(&self) -> u64 {
            self.comparisons
        }
    }
    #[test]
    fn changing_geometry_cannot_issue_stable_pages_when_either_pass_bound_expires() {
        for (maximum, remaining) in [(2, 3), (3, 2), (3, 3)] {
            let mut search = ChangingPages {
                values: [0, 1, 2],
                passes: 0,
                comparisons: 0,
                maximum,
            };
            assert!(matches!(converge(&mut search,remaining),Err(e) if e.kind==E::PagePassLimit));
            assert_eq!(search.passes, usize::from(maximum.min(remaining)));
            assert_eq!(search.comparisons, search.passes as u64 - 1);
        }
        let mut search = ChangingPages {
            values: [0, 1, 1],
            passes: 0,
            comparisons: 0,
            maximum: 3,
        };
        let stable = converge(&mut search, 3).unwrap();
        assert_eq!(
            (stable.sequence, stable.passes, stable.records, stable.work),
            (1, 3, 3, 2)
        );
    }
}

#[cfg(test)]
mod failure_pass_tests {
    use super::*;
    struct FailingSearch {
        started: u16,
        fail_charge: u16,
        fail_select: u16,
        fail_compare: bool,
    }
    impl StableSearch for FailingSearch {
        type Sequence = u16;
        fn maximum_passes(&self) -> u16 {
            4
        }
        fn charge_pass(&mut self) -> Result<(), ProductionBodyPaginationError> {
            if self.started + 1 == self.fail_charge {
                return Err(error(NodeId::new(9), E::FragmentLimit));
            }
            self.started += 1;
            Ok(())
        }
        fn select(&mut self) -> Result<u16, ProductionBodyPaginationError> {
            if self.started == self.fail_select {
                return Err(error(NodeId::new(7), E::PagePassLimit));
            }
            Ok(0)
        }
        fn same(&mut self, _: &u16, _: &u16) -> Result<bool, ProductionBodyPaginationError> {
            if self.fail_compare {
                Err(error(NodeId::new(8), E::FragmentLimit))
            } else {
                Ok(true)
            }
        }
        fn records(&self) -> u64 {
            u64::from(self.started)
        }
        fn work(&self) -> u64 {
            0
        }
    }
    #[test]
    fn page_stability_counted_passes_preserve_selection_comparison_and_charge_failures() {
        for (charge, select, compare, expected) in [
            (1, 0, false, 0),
            (2, 0, false, 1),
            (0, 1, false, 1),
            (0, 2, false, 2),
            (0, 0, true, 2),
        ] {
            let mut search = FailingSearch {
                started: 0,
                fail_charge: charge,
                fail_select: select,
                fail_compare: compare,
            };
            let mut begun = 99;
            let error = converge_counted(&mut search, 4, &mut begun).err().unwrap();
            assert_eq!(begun, expected);
            assert_eq!(search.started, expected);
            assert_eq!(
                error.owner,
                NodeId::new(if charge > 0 {
                    9
                } else if select > 0 {
                    7
                } else {
                    8
                })
            );
        }
        let mut search = FailingSearch {
            started: 0,
            fail_charge: 0,
            fail_select: 0,
            fail_compare: false,
        };
        for remaining in [0, 1] {
            let mut begun = 99;
            assert!(converge_counted(&mut search, remaining, &mut begun).is_err());
            assert_eq!((begun, search.started), (0, 0));
        }
        let mut begun = 0;
        let stable = converge_counted(&mut search, 2, &mut begun).unwrap();
        assert_eq!((stable.passes, begun), (2, 2));
    }
}
