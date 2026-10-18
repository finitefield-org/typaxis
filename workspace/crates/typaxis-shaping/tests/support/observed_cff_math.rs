use super::*;
#[path = "../../../typaxis-font/src/math_validation_cases.rs"]
mod math_cases;
use typaxis_font::{CffAdmissionBudgetV2, CffBaseCmapBudgetV2, CffCmapBudgetV2};
struct MathOnly<'a, 'id, 'cells> {
    inner: BookV2ObservedFontMathBudget<'a, 'id, 'cells>,
    requests: &'a mut Vec<u64>,
}

#[test]
#[ignore = "requires unchanged original MATH CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_math_original_cff_directory_rejection_precedes_math_work() {
    let list = fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = fs::read(list.lines().nth(2).unwrap()).unwrap();
    let digest: String = sha256(&font).iter().map(|n| format!("{n:02x}")).collect();
    assert_eq!(
        digest,
        "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f"
    );
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    let record = font[12..12 + count * 16]
        .chunks_exact(16)
        .find(|r| r[..4] == *b"MATH")
        .unwrap();
    let base = u64::from(u32::from_be_bytes(record[8..12].try_into().unwrap()));
    let length = u64::from(u32::from_be_bytes(record[12..16].try_into().unwrap()));
    assert_eq!((base, length), (689248, 25280));
    let caps = limits();
    let expected =
        typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(font.as_slice()), 0, &caps)
            .unwrap_err();
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
            |a| {
                typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
                    std::sync::Arc::from(font.as_slice()),
                    0,
                    &caps,
                    &mut MathOnly {
                        inner: a.font_math_budget(owner),
                        requests: &mut requests,
                    },
                )
            },
        );
        (result, [r, s, w], requests)
    };
    assert!(matches!(
        expected.kind,
        typaxis_font::Cff1FailureKindV2::Sfnt(typaxis_font::Cff1Error::UnsupportedTable)
    ));
    assert_eq!(
        expected.context.phase,
        typaxis_font::FontFailurePhase::SfntDirectory
    );
    assert_eq!(expected.context.table_tag, Some(*b"FFTM"));
    assert_eq!(expected.context.file_offset, Some(28));
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for extra in [0, 63, 64, MAXIMUM - PRIOR[2]] {
            let (result, cells, requests) = run(owner, extra);
            assert_eq!(result.unwrap().unwrap_err(), expected);
            assert_eq!(cells, PRIOR);
            assert!(requests.is_empty());
        }
    }
    // Preserve the original pre-MATH rejection without issuing a resource receipt.
    eprintln!("original MATH CFF keeps FFTM directory rejection at file offset 28, MATH requests=0, Source cells unchanged");
}
impl CffBaseCmapBudgetV2 for MathOnly<'_, '_, '_> {
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
impl CffCmapBudgetV2 for MathOnly<'_, '_, '_> {
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
impl CffAdmissionBudgetV2 for MathOnly<'_, '_, '_> {
    const OBSERVE_SFNT: bool = false;
    const OBSERVE_MATH: bool = true;
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
    fn work_math(&mut self, n: u64) -> Result<(), Self::Error> {
        self.requests.push(n);
        self.inner.work_math(n)
    }
}
#[test]
fn observed_math_source_boundaries_keep_owner_positions_atomic_work_and_retry() {
    let mut bytes = super::cff_tables::cases::standard();
    for c in math_cases::controls() {
        bytes = super::cff_tables::cases::replace(&bytes, *b"MATH", Some(c.bytes));
    }
    assert_eq!(
        math_cases::cases(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
        )))
        .len(),
        7125
    );
    let caps = limits();
    let expected =
        typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(bytes.as_slice()), 0, &caps)
            .unwrap_err();
    assert!(matches!(
        expected.kind,
        typaxis_font::Cff1FailureKindV2::Program(_)
    ));
    // This structural negative isolates MATH callbacks and issues no font resource.
    let call = |a: &mut BookV2ObservedSourceAccounting<'_, '_>, owner, requests: &mut Vec<u64>| {
        typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
            std::sync::Arc::from(bytes.as_slice()),
            0,
            &caps,
            &mut MathOnly {
                inner: a.font_math_budget(owner),
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
    assert!(requests.iter().all(|n| *n == 64));
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
                    typaxis_font::CffTableAdmissionStageV2::MathHeader
                        | typaxis_font::CffTableAdmissionStageV2::MathField
                        | typaxis_font::CffTableAdmissionStageV2::MathOffset
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
    eprintln!("MATH Source work caps={cases}, requests={}, delta=[0, 0, {amount}], owners=91/205, positions and same-cell retry preserved", requests.len());
}
