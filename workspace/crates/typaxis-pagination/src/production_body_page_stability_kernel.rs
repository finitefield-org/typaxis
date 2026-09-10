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
    let maximum = search.maximum_passes().min(remaining_passes);
    if maximum < 2 {
        return Err(error(NodeId::new(0), E::PagePassLimit));
    }
    search.charge_pass()?;
    let mut previous = search.select()?;
    for pass in 2..=maximum {
        search.charge_pass()?;
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
