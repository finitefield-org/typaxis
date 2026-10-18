use super::super::base_cmap_cases as cases;
use super::super::{validate_cff_cmap_v2, validate_cff_cmap_v2_with_base_cmap_budget};
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Request {
    Reserve(CffBaseCmapStorageV2, usize, usize),
    Work(u64),
}
#[derive(Default)]
struct Visits {
    requests: Vec<Request>,
    accepted: [u64; 3],
    stop: Option<usize>,
}
impl Visits {
    fn request(&mut self, request: Request) -> Result<(), usize> {
        let index = self.requests.len();
        assert!(self.stop.map_or(true, |stop| index <= stop));
        let delta = match request {
            Request::Reserve(_, rows, bytes) => [rows as u64, bytes as u64, 0],
            Request::Work(n) => [0, 0, n],
        };
        self.requests.push(request);
        if self.stop == Some(index) {
            return Err(91);
        }
        for (cell, n) in self.accepted.iter_mut().zip(delta) {
            *cell += n;
        }
        Ok(())
    }
}
impl CffBaseCmapBudgetV2 for Visits {
    type Error = usize;
    fn reserve(
        &mut self,
        site: CffBaseCmapStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), usize> {
        assert!(rows > 0);
        assert_eq!(bytes, rows * std::mem::size_of::<(u32, u16)>());
        self.request(Request::Reserve(site, rows, bytes))
    }
    fn work(&mut self, n: u64) -> Result<(), usize> {
        assert_eq!(n, 1);
        self.request(Request::Work(n))
    }
}
struct Silent;
impl CffBaseCmapBudgetV2 for Silent {
    type Error = usize;
    const OBSERVED: bool = false;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!("ordinary reserve hook")
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        panic!("ordinary work hook")
    }
}
#[test]
fn base_cmap_budget_matches_native_results_and_refuses_every_request_without_refund() {
    let (mut total, mut cutoffs) = (0, 0);
    for (_, bytes, glyphs) in cases::cases() {
        let expected = format!("{:?}", validate_cff_cmap_v2(&bytes, glyphs));
        let mut full = Visits::default();
        let result = validate_cff_cmap_v2_with_base_cmap_budget(&bytes, glyphs, &mut full).unwrap();
        assert_eq!(format!("{result:?}"), expected);
        assert_eq!(
            format!(
                "{:?}",
                validate_cff_cmap_v2_with_base_cmap_budget(&bytes, glyphs, &mut Silent).unwrap()
            ),
            expected
        );
        for stop in 0..full.requests.len() {
            let mut short = Visits {
                stop: Some(stop),
                ..Visits::default()
            };
            let failure =
                validate_cff_cmap_v2_with_base_cmap_budget(&bytes, glyphs, &mut short).unwrap_err();
            assert_eq!(failure.error, 91);
            assert_eq!(failure.file_offset, None);
            assert_eq!(short.requests, full.requests[..=stop]);
            let mut accepted = [0; 3];
            for request in &full.requests[..stop] {
                let delta = match request {
                    Request::Reserve(_, r, s) => [*r as u64, *s as u64, 0],
                    Request::Work(w) => [0, 0, *w],
                };
                for (cell, n) in accepted.iter_mut().zip(delta) {
                    *cell += n;
                }
            }
            assert_eq!(short.accepted, accepted);
            cutoffs += 1;
        }
        total += 1;
    }
    eprintln!(
        "base cmap budget cases={total}, every-request refusal cutoffs={cutoffs}, ordinary hooks=0"
    );
}
#[test]
fn base_cmap_budget_fixed_scalar_visits_include_count_fill_and_notdef_sentinel() {
    let b12 = cases::cmap(&[(3, 10, cases::format12(&[(65, 67, 1)]))]);
    let b4 = cases::cmap(&[(
        3,
        1,
        cases::format4(&[(65, 67, -64, 0), (0xffff, 0xffff, 1, 0)], &[]),
    )]);
    for (bytes, expected) in [(&b12, [3, 24, 13]), (&b4, [3, 24, 17])] {
        let mut budget = Visits::default();
        let cmap = validate_cff_cmap_v2_with_base_cmap_budget(bytes, 8, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(cmap.base_mapping_count(), 3);
        assert_eq!(budget.accepted, expected);
    }
    let mut bytes = b12;
    bytes[14] = 1;
    let mut budget = Visits::default();
    assert!(
        validate_cff_cmap_v2_with_base_cmap_budget(&bytes, 8, &mut budget)
            .unwrap()
            .is_err()
    );
    assert_eq!(budget.accepted, [0, 0, 3]);
}
