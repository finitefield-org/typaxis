use super::super::v2::{
    CffBaseCmapBudgetV2, CffBaseCmapStorageV2, CffCmapBudgetV2, CffVariationStorageV2,
};
use super::*;
#[path = "cff_sfnt_preflight_cases.rs"]
mod cases;
#[derive(Clone, Debug, Eq, PartialEq)]
enum Request {
    Reserve(CffSfntStorageV2, usize, usize),
    Work(u64),
}
impl Request {
    fn delta(&self) -> [u64; 3] {
        match *self {
            Self::Reserve(_, r, s) => [r as u64, s as u64, 0],
            Self::Work(w) => [0, 0, w],
        }
    }
}
#[derive(Default)]
struct Trace {
    requests: Vec<Request>,
    accepted: [u64; 3],
    stop: Option<usize>,
}
impl Trace {
    fn request(&mut self, r: Request) -> Result<(), usize> {
        let i = self.requests.len();
        assert!(self.stop.map_or(true, |stop| i <= stop));
        let delta = r.delta();
        self.requests.push(r);
        if self.stop == Some(i) {
            return Err(91);
        }
        for (v, n) in self.accepted.iter_mut().zip(delta) {
            *v += n;
        }
        Ok(())
    }
}
impl CffBaseCmapBudgetV2 for Trace {
    type Error = usize;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!("SFNT preflight cannot request cmap mappings")
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        panic!("SFNT preflight must use its own work scope")
    }
}
impl CffCmapBudgetV2 for Trace {
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        panic!("SFNT preflight cannot reserve variation bytes")
    }
}
impl CffAdmissionBudgetV2 for Trace {
    fn reserve_sfnt(&mut self, s: CffSfntStorageV2, r: usize, b: usize) -> Result<(), usize> {
        assert_eq!(b, r * std::mem::size_of::<TableRecord>());
        self.request(Request::Reserve(s, r, b))
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), usize> {
        assert!((1..=256).contains(&n));
        self.request(Request::Work(n))
    }
}
fn prefix(requests: &[Request]) -> [u64; 3] {
    let mut out = [0; 3];
    for r in requests {
        for (v, n) in out.iter_mut().zip(r.delta()) {
            *v += n;
        }
    }
    out
}
#[test]
fn sfnt_preflight_budget_matches_native_fields_and_refuses_every_request() {
    let limits = typaxis_core::M4ResourceLimits::default();
    let (mut fixtures, mut cuts) = (0, 0);
    for (_, bytes) in cases::cases() {
        for vertical in [false, true] {
            let mut expected_context = FontFailureContext::new(0);
            let expected = ordinary(&bytes, limits, &mut expected_context, vertical);
            let mut context = FontFailureContext::new(0);
            let mut full = Trace::default();
            let actual = with_budget(&bytes, limits, &mut context, vertical, &mut full)
                .unwrap()
                .map_err(|e| match e {
                    NativeFailure::Native(e) => e,
                    other => panic!("unexpected allocation failure: {other:?}"),
                });
            assert_eq!(actual, expected);
            assert_eq!(context, expected_context);
            for stop in 0..full.requests.len() {
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let error = with_budget(
                    &bytes,
                    limits,
                    &mut FontFailureContext::new(0),
                    vertical,
                    &mut short,
                )
                .unwrap_err();
                assert_eq!(error.error, 91);
                assert_eq!(short.requests, full.requests[..=stop]);
                assert_eq!(short.accepted, prefix(&full.requests[..stop]));
                if error.point.stage == CffSfntStageV2::DirectoryOrder {
                    assert_eq!(error.point.file_offset, None);
                }
                cuts += 1;
            }
            fixtures += 1;
        }
    }
    eprintln!(
        "SFNT preflight native/context cases={fixtures}, every-request refusal cutoffs={cuts}"
    );
}
#[test]
fn sfnt_preflight_checksum_batches_match_independent_tail_and_head_rules() {
    let (mut count, mut cuts) = (0, 0);
    for n in [
        0usize, 1, 2, 3, 4, 7, 8, 11, 12, 13, 63, 64, 65, 127, 128, 255, 256, 257, 511, 512, 513,
        1027,
    ] {
        let bytes = (0..n)
            .map(|i| ((i * 73 + 17) % 251) as u8)
            .collect::<Vec<_>>();
        for head in [false, true] {
            if head && n < 12 {
                continue;
            }
            let mut reference = bytes.clone();
            if head {
                reference[8..12].fill(0);
            }
            let expected = cases::checksum(&reference);
            let mut full = Trace::default();
            assert_eq!(
                checksum_observed(
                    &bytes,
                    Some(*b"head"),
                    321,
                    head,
                    CffSfntStageV2::TableChecksum,
                    &mut full
                )
                .unwrap(),
                expected
            );
            assert_eq!(full.accepted, [0, 0, n.div_ceil(4) as u64]);
            assert_eq!(full.requests.len(), n.div_ceil(256));
            for stop in 0..full.requests.len() {
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let e = checksum_observed(
                    &bytes,
                    Some(*b"head"),
                    321,
                    head,
                    CffSfntStageV2::TableChecksum,
                    &mut short,
                )
                .unwrap_err();
                assert_eq!(e.error, 91);
                assert_eq!(e.point.file_offset, Some((321 + stop * 256) as u64));
                assert_eq!(e.point.table_offset, Some((stop * 256) as u64));
                assert_eq!(short.accepted, prefix(&full.requests[..stop]));
                assert_eq!(short.requests, full.requests[..=stop]);
                cuts += 1;
            }
            count += 1;
        }
    }
    eprintln!(
        "SFNT independent checksum cases={count}, batch refusals={cuts}, checksum heap requests=0"
    );
}
#[test]
fn sfnt_preflight_directory_allocations_are_fallible_without_refunding_requests() {
    let bytes = cases::fixture(4, true, 1, false);
    let limits = typaxis_core::M4ResourceLimits::default();
    let mut full = Trace::default();
    assert!(with_budget(
        &bytes,
        limits,
        &mut FontFailureContext::new(0),
        true,
        &mut full
    )
    .unwrap()
    .is_ok());
    let mut count = 0;
    for stop in 0..2 {
        let mut trace = Trace::default();
        let mut calls = 0;
        let result = with_policy(
            &bytes,
            limits,
            &mut FontFailureContext::new(0),
            Policy::legacy(true),
            &mut trace,
            &mut |rows, n| {
                let i = calls;
                calls += 1;
                rows.try_reserve_exact(if i == stop { usize::MAX } else { n })
                    .map_err(|_| ())
            },
        );
        assert_eq!(result.unwrap(), Err(NativeFailure::Allocation));
        assert_eq!(calls, stop + 1);
        assert_eq!(trace.accepted[0], 9 * (stop + 1) as u64);
        assert_eq!(
            trace.accepted[1],
            9 * (stop + 1) as u64 * std::mem::size_of::<TableRecord>() as u64
        );
        assert_eq!(trace.requests, &full.requests[..trace.requests.len()]);
        count += 1;
    }
    eprintln!("SFNT exact directory allocations=2, real capacity-overflow failures={count}");
}
#[test]
fn sfnt_preflight_order_matches_stable_offset_sort_including_equal_offsets() {
    let (mut count, mut cuts) = (0, 0);
    for n in [0usize, 1, 2, 3, 4, 8, 16, 31, 64, 128, 256] {
        for seed in 0..5u64 {
            let mut x = seed + 7;
            let rows = (0..n)
                .map(|i| {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    TableRecord {
                        tag: (i as u32).to_be_bytes(),
                        offset: (x >> 32) as usize % 17,
                        length: i + 1,
                        padded_end: i + 4,
                    }
                })
                .collect::<Vec<_>>();
            let mut expected = rows.clone();
            expected.sort_by_key(|r| r.offset);
            let mut actual = rows.clone();
            let mut full = Trace::default();
            sort_records(&mut actual, &mut full).unwrap();
            assert_eq!(actual, expected);
            let selected: Vec<_> = if n <= 16 {
                (0..full.requests.len()).collect()
            } else {
                vec![0, full.requests.len() / 2, full.requests.len() - 1]
            };
            for stop in selected {
                let mut actual = rows.clone();
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let e = sort_records(&mut actual, &mut short).unwrap_err();
                assert_eq!(e.error, 91);
                assert_eq!(e.point.file_offset, None);
                assert_eq!(e.point.stage, CffSfntStageV2::DirectoryOrder);
                assert_eq!(short.requests, full.requests[..=stop]);
                cuts += 1;
            }
            count += 1;
        }
    }
    eprintln!("SFNT independent stable-order cases={count}, refusal cutoffs={cuts}");
}
