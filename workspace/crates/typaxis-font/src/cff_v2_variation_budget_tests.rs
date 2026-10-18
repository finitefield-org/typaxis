use super::super::variation_budget_cases as cases;
use super::super::{
    validate_cff_cmap_v2, validate_cff_cmap_v2_with_cmap_budget, CffBaseCmapBudgetV2,
    CffBaseCmapStorageV2, CffCmapBudgetFailureV2, CffCmapBudgetV2, CffVariationStorageV2,
};
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Request {
    Base(CffBaseCmapStorageV2, usize, usize),
    Variation(CffVariationStorageV2, usize, usize),
    Work(u64),
}
impl Request {
    fn delta(&self) -> [u64; 3] {
        match *self {
            Self::Base(_, r, s) | Self::Variation(_, r, s) => [r as u64, s as u64, 0],
            Self::Work(w) => [0, 0, w],
        }
    }
}
#[derive(Default)]
struct Visits {
    requests: Vec<Request>,
    accepted: [u64; 3],
    stop: Option<usize>,
}
impl Visits {
    fn request(&mut self, r: Request) -> Result<(), usize> {
        let at = self.requests.len();
        assert!(self.stop.map_or(true, |stop| at <= stop));
        let delta = r.delta();
        self.requests.push(r);
        if self.stop == Some(at) {
            return Err(205);
        }
        for (v, n) in self.accepted.iter_mut().zip(delta) {
            *v += n;
        }
        Ok(())
    }
}
impl CffBaseCmapBudgetV2 for Visits {
    type Error = usize;
    fn reserve(&mut self, site: CffBaseCmapStorageV2, r: usize, s: usize) -> Result<(), usize> {
        assert_eq!(s, r * std::mem::size_of::<(u32, u16)>());
        self.request(Request::Base(site, r, s))
    }
    fn work(&mut self, n: u64) -> Result<(), usize> {
        assert!((1..=256).contains(&n));
        self.request(Request::Work(n))
    }
}
impl CffCmapBudgetV2 for Visits {
    fn reserve_variation(
        &mut self,
        site: CffVariationStorageV2,
        r: usize,
        s: usize,
    ) -> Result<(), usize> {
        match site {
            CffVariationStorageV2::PayloadSpans => {
                assert_eq!(s, r * std::mem::size_of::<(usize, usize, u8)>())
            }
            CffVariationStorageV2::OwnedBytes => assert_eq!(r, 0),
        }
        self.request(Request::Variation(site, r, s))
    }
}
struct Silent;
impl CffBaseCmapBudgetV2 for Silent {
    type Error = usize;
    const OBSERVED: bool = false;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!("ordinary reserve")
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        panic!("ordinary work")
    }
}
impl CffCmapBudgetV2 for Silent {
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        panic!("ordinary variation reserve")
    }
}
fn error(f: CffCmapBudgetFailureV2<usize>) -> usize {
    match f {
        CffCmapBudgetFailureV2::Base(e) => e.error,
        CffCmapBudgetFailureV2::Variation(e) => {
            assert_eq!(e.file_offset, None);
            e.error
        }
    }
}
#[test]
fn cmap_full_budget_matches_native_and_stops_at_every_small_fixture_request() {
    let mut fixtures = cases::cases();
    fixtures.extend(super::super::base_cmap_cases::cases());
    let (mut count, mut cutoffs, mut variation_cuts) = (0, 0, 0);
    for (_, bytes, glyphs) in fixtures {
        let expected = format!("{:?}", validate_cff_cmap_v2(&bytes, glyphs));
        let mut full = Visits::default();
        assert_eq!(
            format!(
                "{:?}",
                validate_cff_cmap_v2_with_cmap_budget(&bytes, glyphs, &mut full).unwrap()
            ),
            expected
        );
        assert_eq!(
            format!(
                "{:?}",
                validate_cff_cmap_v2_with_cmap_budget(&bytes, glyphs, &mut Silent).unwrap()
            ),
            expected
        );
        if bytes.len() <= 300 {
            for stop in 0..full.requests.len() {
                let mut short = Visits {
                    stop: Some(stop),
                    ..Visits::default()
                };
                let failure =
                    validate_cff_cmap_v2_with_cmap_budget(&bytes, glyphs, &mut short).unwrap_err();
                variation_cuts +=
                    usize::from(matches!(failure, CffCmapBudgetFailureV2::Variation(_)));
                assert_eq!(error(failure), 205);
                assert_eq!(short.requests, full.requests[..=stop]);
                let mut prefix = [0; 3];
                for r in &full.requests[..stop] {
                    for (v, n) in prefix.iter_mut().zip(r.delta()) {
                        *v += n;
                    }
                }
                assert_eq!(short.accepted, prefix);
                cutoffs += 1;
            }
        }
        count += 1;
    }
    eprintln!("full cmap budget cases={count}, every-request cutoffs={cutoffs}, variation cutoffs={variation_cuts}, ordinary hooks=0");
}
#[test]
fn cmap_full_budget_heapsort_matches_independent_order_and_refuses_comparisons_and_swaps() {
    let (mut cases, mut cuts) = (0, 0);
    for n in [0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 256, 512] {
        for seed in 0..5u64 {
            let mut x = seed + 91;
            let input = (0..n)
                .map(|_| {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let a = (x >> 32) as usize % 37;
                    (a, a + ((x >> 16) as usize % 11), (x % 2) as u8)
                })
                .collect::<Vec<_>>();
            let mut expected = input.clone();
            expected.sort_unstable();
            let mut actual = input.clone();
            let mut full = Visits::default();
            sort_spans(
                &mut actual,
                &mut VariationObserver {
                    budget: &mut full,
                    table_start: 17,
                },
            )
            .unwrap();
            assert_eq!(actual, expected);
            let selected: Vec<_> = if n <= 16 {
                (0..full.requests.len()).collect()
            } else {
                vec![0, full.requests.len() / 2, full.requests.len() - 1]
            };
            for stop in selected {
                let mut actual = input.clone();
                let mut short = Visits {
                    stop: Some(stop),
                    ..Visits::default()
                };
                let failure = sort_spans(
                    &mut actual,
                    &mut VariationObserver {
                        budget: &mut short,
                        table_start: 17,
                    },
                )
                .unwrap_err();
                assert_eq!(error(failure), 205);
                assert_eq!(short.requests, full.requests[..=stop]);
                cuts += 1;
            }
            cases += 1;
        }
    }
    eprintln!("variation heapsort independent cases={cases}, refusal cutoffs={cuts}");
}
#[test]
fn cmap_full_budget_empty_base_pays_usable_scan_and_owned_copy() {
    let mut b = cases::cmap(&cases::table(1, false, false), true);
    b.drain(36..48);
    b[24..28].copy_from_slice(&16u32.to_be_bytes());
    b[32..36].fill(0);
    b[16..20].copy_from_slice(&36u32.to_be_bytes());
    let mut full = Visits::default();
    let p = validate_cff_cmap_v2_with_cmap_budget(&b, 8, &mut full)
        .unwrap()
        .unwrap();
    assert_eq!(p.base_mapping_count(), 0);
    assert_eq!(p.glyph_for_sequence('\u{4e03}', Some('\u{e0100}')), Some(4));
    let mut bad = b.clone();
    *bad.last_mut().unwrap() = 0;
    let mut no_gid = Visits::default();
    assert!(validate_cff_cmap_v2_with_cmap_budget(&bad, 8, &mut no_gid)
        .unwrap()
        .is_err());
    assert!(!no_gid.requests.iter().any(|r| matches!(
        r,
        Request::Variation(CffVariationStorageV2::OwnedBytes, _, _)
    )));
    assert!(full.requests.iter().any(|r| matches!(
        r,
        Request::Variation(CffVariationStorageV2::OwnedBytes, 0, 38)
    )));
    for stop in 0..full.requests.len() {
        let mut short = Visits {
            stop: Some(stop),
            ..Visits::default()
        };
        assert_eq!(
            error(validate_cff_cmap_v2_with_cmap_budget(&b, 8, &mut short).unwrap_err()),
            205
        );
        assert_eq!(short.requests, full.requests[..=stop]);
    }
    eprintln!(
        "empty-base usable-pair and owned-copy cutoffs={}",
        full.requests.len()
    );
}
