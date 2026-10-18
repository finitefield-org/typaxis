use super::*;
#[path = "../../../typaxis-font/src/cff_v2_program_cases.rs"]
mod cases;
use typaxis_font::{CffAdmissionBudgetV2, CffBaseCmapBudgetV2, CffCmapBudgetV2};
struct Relay<'a, 'id, 'cells> {
    inner: BookV2ObservedFontProgramBudget<'a, 'id, 'cells>,
    requests: &'a mut Vec<[u64; 3]>,
}
impl CffBaseCmapBudgetV2 for Relay<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    fn reserve(
        &mut self,
        _: typaxis_font::CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        panic!()
    }
    fn work(&mut self, _: u64) -> Result<(), Self::Error> {
        panic!()
    }
}
impl CffCmapBudgetV2 for Relay<'_, '_, '_> {
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        panic!()
    }
}
impl CffAdmissionBudgetV2 for Relay<'_, '_, '_> {
    const OBSERVE_PROGRAM: bool = true;
    fn reserve_sfnt(
        &mut self,
        _: typaxis_font::CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        panic!()
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), Self::Error> {
        panic!()
    }
    fn reserve_program(
        &mut self,
        site: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.requests.push([r as u64, b as u64, 0]);
        self.inner.reserve_program(site, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.requests.push([0, 0, n]);
        self.inner.work_program(n)
    }
}
#[test]
fn observed_program_source_request_boundaries_keep_real_cells_owners_and_atomic_prefix() {
    let bytes = cases::cid_fixture(true);
    assert!(!cases::cases().is_empty());
    let base = limits();
    let run = |owner, axis: usize, extra: u64| {
        let mut raw = base.base().get().clone();
        if axis == 1 {
            raw.max_spool_bytes = PRIOR[1] + extra;
        }
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        let mut requests = Vec::new();
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &domain,
            if axis == 0 { PRIOR[0] + extra } else { MAXIMUM },
            if axis == 2 { PRIOR[2] + extra } else { MAXIMUM },
            |a| {
                typaxis_font::inspect_cff1_program_v2_with_budget(
                    std::sync::Arc::from(bytes.as_slice()),
                    3,
                    1000,
                    5,
                    &mut Relay {
                        inner: a.font_program_budget(owner),
                        requests: &mut requests,
                    },
                )
            },
        );
        (result, [r, s, w], requests)
    };
    let (ready, full, requests) = run(NodeId::new(91), 3, 0);
    let parsed = ready.unwrap().unwrap();
    assert_eq!(parsed.font_dict_count(), 2);
    assert_eq!(parsed.width_defaults(1), Some((600 * 65536, 0)));
    // This is raw program inspection; it does not issue a font resource receipt.
    let delta = [full[0] - PRIOR[0], full[1] - PRIOR[1], full[2] - PRIOR[2]];
    let mut cases = 0;
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for axis in 0..3 {
            let mut values = std::collections::BTreeSet::from([
                0,
                1,
                delta[axis] - 1,
                delta[axis],
                delta[axis] + 1,
            ]);
            let mut prefix = 0;
            for r in &requests {
                prefix += r[axis];
                values.insert(prefix);
                if prefix > 0 {
                    values.insert(prefix - 1);
                }
                values.insert(prefix + 1);
            }
            for extra in values {
                let (result, cells, seen) = run(owner, axis, extra);
                if extra >= delta[axis] {
                    result.unwrap().unwrap();
                    assert_eq!(cells, full);
                    assert_eq!(seen, requests);
                } else {
                    let f = result.unwrap_err();
                    assert_eq!(
                        f.error,
                        match axis {
                            0 => BookV2ObservedAccountingError::Records { owner },
                            1 => BookV2ObservedAccountingError::Spool { owner },
                            _ => BookV2ObservedAccountingError::Work { owner },
                        }
                    );
                    assert_eq!(seen, requests[..seen.len()]);
                    let accepted = &seen[..seen.len() - 1];
                    let mut expected = PRIOR;
                    for r in accepted {
                        for (v, n) in expected.iter_mut().zip(r) {
                            *v += *n;
                        }
                    }
                    assert_eq!(cells, expected);
                    assert!(cells[axis] <= PRIOR[axis] + extra);
                    assert!(f
                        .point
                        .table_offset
                        .map_or(true, |n| n <= bytes.len() as u64));
                }
                cases += 1;
            }
        }
    }
    eprintln!("program Source boundary caps={cases}, requests={}, delta={delta:?}, owners=91/205, all-axis atomic prefixes preserved",requests.len());
}
