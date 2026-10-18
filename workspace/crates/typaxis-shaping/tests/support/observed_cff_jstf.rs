use super::*;
#[allow(dead_code)]
#[path = "../../../typaxis-font/src/cff_jstf_cases.rs"]
mod jstf_cases;
use typaxis_font::{CffAdmissionBudgetV2, CffBaseCmapBudgetV2, CffCmapBudgetV2};
struct JstfOnly<'a, 'id, 'cells> {
    inner: BookV2ObservedFontJstfBudget<'a, 'id, 'cells>,
    requests: &'a mut Vec<u64>,
}
impl CffBaseCmapBudgetV2 for JstfOnly<'_, '_, '_> {
    type Error = BookV2ObservedAccountingError;
    const OBSERVED: bool = false;
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
impl CffCmapBudgetV2 for JstfOnly<'_, '_, '_> {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        panic!()
    }
}
impl CffAdmissionBudgetV2 for JstfOnly<'_, '_, '_> {
    const OBSERVE_SFNT: bool = false;
    const OBSERVE_JSTF: bool = true;
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
    fn work_jstf(&mut self, n: u64) -> Result<(), Self::Error> {
        self.requests.push(n);
        self.inner.work_jstf(n)
    }
}
#[test]
fn observed_jstf_source_boundaries_keep_owner_positions_atomic_work_and_retry() {
    let mut bytes = super::cff_tables::cases::standard();
    let (_, lookup) = jstf_cases::lookup_seeds()
        .into_iter()
        .find(|(n, _)| n == "context2-true-1-2")
        .unwrap();
    bytes = super::cff_tables::cases::replace(&bytes, *b"JSTF", Some(jstf_cases::jstf(&lookup)));
    let caps = limits();
    let expected =
        typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(bytes.as_slice()), 0, &caps)
            .unwrap_err();
    assert!(matches!(
        expected.kind,
        typaxis_font::Cff1FailureKindV2::Program(_)
    ));
    // This structural negative isolates JSTF callbacks and issues no font resource.
    let call = |a: &mut BookV2ObservedSourceAccounting<'_, '_>, owner, requests: &mut Vec<u64>| {
        typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
            std::sync::Arc::from(bytes.as_slice()),
            0,
            &caps,
            &mut JstfOnly {
                inner: a.font_jstf_budget(owner),
                requests,
            },
        )
    };
    let run = |owner, extra: u64| {
        let [mut r, mut s, mut w] = PRIOR;
        let mut requests = Vec::new();
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            PRIOR[0],
            PRIOR[2] + extra,
            |a| call(a, owner, &mut requests),
        );
        (result, [r, s, w], requests)
    };
    let (ready, full, requests) = run(NodeId::new(91), MAXIMUM - PRIOR[2]);
    assert_eq!(ready.unwrap().unwrap_err(), expected);
    assert_eq!(full[..2], PRIOR[..2]);
    assert!(requests.iter().all(|n| [64, 512, 65536].contains(n)));
    assert!(requests.contains(&65536));
    let amount = full[2] - PRIOR[2];
    let mut values = std::collections::BTreeSet::from([0, 1, amount - 1, amount, amount + 1]);
    let mut prefix = 0;
    for n in &requests {
        prefix += n;
        values.extend([prefix - 1, prefix, prefix + 1]);
    }
    let mut cases = 0;
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for extra in &values {
            let (result, cells, seen) = run(owner, *extra);
            assert_eq!(cells[..2], PRIOR[..2]);
            if *extra >= amount {
                assert_eq!(result.unwrap().unwrap_err(), expected);
                assert_eq!(cells, full);
                assert_eq!(seen, requests);
            } else {
                let typaxis_font::CffAdmissionBudgetFailureV2::Tables(f) = result.unwrap_err()
                else {
                    panic!()
                };
                assert_eq!(f.error, BookV2ObservedAccountingError::Work { owner });
                assert!(matches!(
                    f.point.stage,
                    typaxis_font::CffTableAdmissionStageV2::JstfHeader
                        | typaxis_font::CffTableAdmissionStageV2::JstfField
                        | typaxis_font::CffTableAdmissionStageV2::JstfOffset
                        | typaxis_font::CffTableAdmissionStageV2::JstfRange
                        | typaxis_font::CffTableAdmissionStageV2::JstfReader
                        | typaxis_font::CffTableAdmissionStageV2::JstfVisit
                        | typaxis_font::CffTableAdmissionStageV2::JstfSetInit
                ));
                let tag = f.point.table_tag.unwrap();
                let count = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
                let record = bytes[12..12 + count * 16]
                    .chunks_exact(16)
                    .find(|r| r[..4] == tag)
                    .unwrap();
                let base = u64::from(u32::from_be_bytes(record[8..12].try_into().unwrap()));
                let length = u64::from(u32::from_be_bytes(record[12..16].try_into().unwrap()));
                assert!(f.point.table_offset.map_or(true, |n| n <= length));
                assert_eq!(f.point.file_offset, f.point.table_offset.map(|n| base + n));
                assert_eq!(seen, requests[..seen.len()]);
                assert_eq!(
                    cells[2],
                    PRIOR[2] + seen[..seen.len() - 1].iter().sum::<u64>()
                );
                assert!(cells[2] <= PRIOR[2] + extra);
            }
            cases += 1;
        }
        let [mut r, mut s, mut w] = PRIOR;
        with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            PRIOR[0],
            full[2] - 1,
            |a| {
                let mut seen = Vec::new();
                assert!(call(a, owner, &mut seen).is_err());
                let paid = a.counters();
                assert!(paid[2] > PRIOR[2]);
                seen.clear();
                assert!(call(a, owner, &mut seen).is_err());
                assert_eq!(seen, [64]);
                assert_eq!(a.counters(), paid);
            },
        );
    }
    eprintln!("JSTF Source work caps={cases}, requests={}, delta=[0, 0, {amount}], owners=91/205, positions and same-cell retry preserved", requests.len());
}
