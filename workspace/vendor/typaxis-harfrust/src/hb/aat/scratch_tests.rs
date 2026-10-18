use super::*;
use crate::scratch::{Budget, Error, ObservePlan, PlanScratch, Site};
use crate::{Direction, Feature, UnicodeBuffer};
use alloc::{format, vec, vec::Vec};

#[path = "../../../test-support/aat_font.rs"]
mod fixture;
const BASE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));

#[derive(Clone, Debug, Eq, PartialEq)]
struct Request {
    site: Site,
    count: usize,
    bytes: usize,
    spool: usize,
    work: usize,
}
#[derive(Default)]
struct Meter {
    requests: Vec<Request>,
    spool: usize,
    work: usize,
    fail_at: Option<usize>,
    maximum_work: Option<usize>,
    failed: bool,
}
impl Budget for Meter {
    fn reserve(&mut self, site: Site, count: usize, bytes: usize) -> Result<(), Error> {
        assert!(!self.failed);
        self.work(count)?;
        self.spool += count.checked_mul(bytes).unwrap();
        self.requests.push(Request {
            site,
            count,
            bytes,
            spool: self.spool,
            work: self.work,
        });
        if self.fail_at == Some(self.requests.len() - 1) {
            self.failed = true;
            return Err(Error::AllocationFailure);
        }
        Ok(())
    }
    fn work(&mut self, count: usize) -> Result<(), Error> {
        assert!(!self.failed);
        let next = self.work.checked_add(count).unwrap();
        if self.maximum_work.is_some_and(|maximum| next > maximum) {
            self.failed = true;
            return Err(Error::Refused);
        }
        self.work = next;
        Ok(())
    }
}
fn input(text: &str, direction: Direction) -> UnicodeBuffer {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_language("en".parse().unwrap());
    buffer.guess_segment_properties();
    buffer.set_direction(direction);
    buffer
}
fn features(rows: &[&str]) -> Vec<Feature> {
    rows.iter().map(|row| row.parse().unwrap()).collect()
}
fn fixture_font(lower: bool) -> Vec<u8> {
    crate::test_font::with_replaced_tables(BASE, &fixture::aat_tables(3, lower))
}

#[test]
fn aat_scratch_stable_sort_pays_indexed_records_and_keeps_original_values_on_refusal() {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Tagged(u32, u32);
    impl Ord for Tagged {
        fn cmp(&self, other: &Self) -> Ordering {
            self.0.cmp(&other.0)
        }
    }
    impl PartialOrd for Tagged {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    let original = (0..24).map(|i| Tagged((i * 17) % 7, i)).collect::<Vec<_>>();
    let mut expected = original.clone();
    expected.sort();
    let run = |budget: &mut Meter, values: &mut [Tagged]| {
        let mut observer = ObservePlan(budget);
        PlanScratch::new(Some(&mut observer)).stable_sort(values)
    };
    let mut full = Meter::default();
    let mut values = original.clone();
    run(&mut full, &mut values).unwrap();
    assert_eq!(values, expected);
    assert_eq!(full.requests.len(), 1);
    assert_eq!(full.requests[0].site, Site::AatSortRecords);
    assert_eq!(full.requests[0].count, original.len());
    for maximum in 0..full.work {
        let mut budget = Meter {
            maximum_work: Some(maximum),
            ..Meter::default()
        };
        let mut values = original.clone();
        assert_eq!(run(&mut budget, &mut values), Err(Error::Refused));
        assert_eq!(values, original);
        assert!(budget.work <= maximum);
    }
    let mut budget = Meter {
        fail_at: Some(0),
        ..Meter::default()
    };
    let mut values = original.clone();
    assert_eq!(run(&mut budget, &mut values), Err(Error::AllocationFailure));
    assert_eq!(values, original);
    let mut exact = Meter {
        maximum_work: Some(full.work),
        ..Meter::default()
    };
    run(&mut exact, &mut values).unwrap();
    assert_eq!(values, expected);
    std::eprintln!(
        "AAT stable indexed sort: records=24, paid faults=1, work cutoffs={}, spool={}",
        full.work,
        full.spool
    );
}

#[test]
fn aat_scratch_actual_shaping_matches_immutable_registry_for_ties_ranges_and_fallback() {
    let cases: &[&[&str]] = &[
        &[],
        &["liga=0"],
        &["liga=0", "liga=1"],
        &["liga=1", "liga=0"],
        &["liga[0:3]=0", "liga[0:3]=1"],
        &["liga[2:6]=1", "liga[0:4]=0"],
        &["smcp=1", "aalt=1", "dlig=1"],
        &["liga[2:2]=0"],
        &[
            "liga=0",
            "smcp[1:5]=1",
            "liga[3:7]=1",
            "aalt=1",
            "dlig[0:4]=0",
        ],
    ];
    let mut count = 0;
    for lower in [false, true] {
        let bytes = fixture_font(lower);
        let font = crate::FontRef::new(&bytes).unwrap();
        let data = crate::ShaperData::new(&font);
        let reference = harfrust_reference::ShaperData::new(&font);
        for direction in [Direction::LeftToRight, Direction::RightToLeft] {
            for text in ["111111", "(111) a1"] {
                for rows in cases {
                    let features = features(rows);
                    let actual = data
                        .shaper(&font)
                        .build()
                        .try_shape_with_scratch(
                            input(text, direction),
                            &features,
                            &mut Meter::default(),
                        )
                        .unwrap();
                    let mut buffer = harfrust_reference::UnicodeBuffer::new();
                    buffer.push_str(text);
                    buffer.set_language("en".parse().unwrap());
                    buffer.guess_segment_properties();
                    buffer.set_direction(if direction == Direction::LeftToRight {
                        harfrust_reference::Direction::LeftToRight
                    } else {
                        harfrust_reference::Direction::RightToLeft
                    });
                    let reference_features: Vec<harfrust_reference::Feature> =
                        rows.iter().map(|row| row.parse().unwrap()).collect();
                    let expected = reference
                        .shaper(&font)
                        .build()
                        .shape(buffer, &reference_features);
                    assert_eq!(
                        format!("{:?}", actual.glyph_infos()),
                        format!("{:?}", expected.glyph_infos()),
                        "lower={lower} {direction:?} {rows:?}"
                    );
                    assert_eq!(
                        format!("{:?}", actual.glyph_positions()),
                        format!("{:?}", expected.glyph_positions())
                    );
                    if rows.is_empty() {
                        assert!(actual.glyph_infos().iter().any(|info| info.glyph_id == 19));
                    }
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 72);
    std::eprintln!("AAT immutable-registry derived-font comparisons={count}");
}

#[test]
fn aat_scratch_every_actual_reservation_and_work_cutoff_stops_before_glyph_publication() {
    let bytes = fixture_font(false);
    let font = crate::FontRef::new(&bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let features = features(&[
        "liga=0",
        "liga[2:6]=1",
        "smcp[1:7]=1",
        "aalt=1",
        "dlig[3:5]=1",
        "liga[4:8]=0",
    ]);
    let buffer = input("(111) a1", Direction::LeftToRight);
    let language = buffer.language();
    let plan = crate::ShapePlan::new(
        &face,
        buffer.direction(),
        Some(buffer.script()),
        language.as_ref(),
        &features,
    );
    assert!(plan.apply_morx);
    let run = |budget: &mut Meter| {
        face.try_shape_with_plan_and_scratch(
            &plan,
            input("(111) a1", Direction::LeftToRight),
            &features,
            budget,
        )
    };
    let mut full = Meter::default();
    let expected = run(&mut full).unwrap();
    for site in [
        Site::AatFeatures,
        Site::AatFeatureEvents,
        Site::AatActiveFeatures,
        Site::AatCurrentFeatures,
        Site::AatChainFlags,
        Site::AatRangeFlags,
        Site::AatSortRecords,
    ] {
        assert!(full.requests.iter().any(|r| r.site == site), "{site:?}");
    }
    for index in 0..full.requests.len() {
        let mut budget = Meter {
            fail_at: Some(index),
            ..Meter::default()
        };
        assert!(matches!(run(&mut budget), Err(Error::AllocationFailure)));
        assert_eq!(budget.requests, full.requests[..=index]);
        assert_eq!(
            (budget.spool, budget.work),
            (full.requests[index].spool, full.requests[index].work)
        );
    }
    for maximum in 0..full.work {
        let mut budget = Meter {
            maximum_work: Some(maximum),
            ..Meter::default()
        };
        assert!(
            matches!(run(&mut budget), Err(Error::Refused)),
            "cap={maximum}"
        );
        assert!(budget.work <= maximum);
        assert_eq!(budget.requests, full.requests[..budget.requests.len()]);
    }
    let mut exact = Meter {
        maximum_work: Some(full.work),
        ..Meter::default()
    };
    let actual = run(&mut exact).unwrap();
    assert_eq!(
        format!("{:?}", actual.glyph_infos()),
        format!("{:?}", expected.glyph_infos())
    );
    std::eprintln!(
        "AAT actual map: sites=7, paid faults={}, work cutoffs={}, spool={}",
        full.requests.len(),
        full.work,
        full.spool
    );
}

#[test]
fn aat_scratch_compiled_ranges_preserve_stable_tie_winner_and_default_flags() {
    let bytes = fixture_font(false);
    let font = crate::FontRef::new(&bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    for (rows, expected) in [
        (vec![], vec![(1, 0, u32::MAX)]),
        (vec!["liga=0", "liga=1"], vec![(0, 0, u32::MAX)]),
        (vec!["liga=1", "liga=0"], vec![(1, 0, u32::MAX)]),
        (
            vec!["liga[0:3]=0", "liga[0:3]=1"],
            vec![(0, 0, 2), (1, 3, u32::MAX)],
        ),
    ] {
        let mut builder = AatMapBuilder::default();
        let mut map = AatMap::default();
        let mut budget = Meter::default();
        let mut observer = ObservePlan(&mut budget);
        let mut scratch = PlanScratch::new(Some(&mut observer));
        for feature in features(&rows) {
            builder
                .add_feature_with_scratch(&face, &feature, &mut scratch)
                .unwrap();
        }
        builder
            .compile_with_scratch(&face, &mut map, &mut scratch)
            .unwrap();
        assert_eq!(map.chain_flags.len(), 3);
        for chain in &map.chain_flags {
            assert_eq!(
                chain
                    .iter()
                    .map(|r| (r.flags, r.cluster_first, r.cluster_last))
                    .collect::<Vec<_>>(),
                expected,
                "{rows:?}"
            );
        }
    }
}

#[test]
fn aat_scratch_owned_outputs_keep_non_send_budgets_and_native_dispatch_separate() {
    use std::{cell::Cell, rc::Rc};
    struct Local(Rc<Cell<usize>>);
    impl Budget for Local {
        fn reserve(&mut self, _: Site, count: usize, bytes: usize) -> Result<(), Error> {
            self.0.set(self.0.get() + count * bytes);
            Ok(())
        }
        fn work(&mut self, _: usize) -> Result<(), Error> {
            Ok(())
        }
    }
    struct Native;
    impl Budget for Native {
        const OBSERVED: bool = false;
        fn reserve(&mut self, _: Site, _: usize, _: usize) -> Result<(), Error> {
            panic!("native reserve")
        }
        fn work(&mut self, _: usize) -> Result<(), Error> {
            panic!("native work")
        }
    }
    fn owned<T: Send + Sync + 'static>(_: &T) {}
    let bytes = fixture_font(true);
    let font = crate::FontRef::new(&bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let count = Rc::new(Cell::new(0));
    let glyphs = {
        let mut local = Local(count.clone());
        face.try_shape_with_scratch(input("111", Direction::LeftToRight), &[], &mut local)
            .unwrap()
    };
    owned(&glyphs);
    assert!(count.get() > 0);
    assert!(glyphs.glyph_infos().iter().all(|g| g.glyph_id == 19));
    let native = face
        .try_shape_with_scratch(input("111", Direction::LeftToRight), &[], &mut Native)
        .unwrap();
    assert_eq!(
        format!("{:?}", native.glyph_infos()),
        format!("{:?}", glyphs.glyph_infos())
    );
    let font = crate::FontRef::new(BASE).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let mut budget = Meter::default();
    let mut observer = ObservePlan(&mut budget);
    let mut scratch = PlanScratch::new(Some(&mut observer));
    assert_eq!(
        AatMapBuilder::default().add_feature_with_scratch(
            &face,
            &features(&["liga=0"])[0],
            &mut scratch
        ),
        Ok(None)
    );
    assert_eq!(budget.spool, 0);
    assert_eq!(budget.work, 0);
}
