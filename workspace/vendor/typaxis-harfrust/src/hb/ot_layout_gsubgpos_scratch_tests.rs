use crate::scratch::{Budget, Error, ObservePlan, PlanScratch, Site};
use crate::{Direction, ShaperData, UnicodeBuffer};
use alloc::{format, vec, vec::Vec};

#[path = "../../test-support/contextual_font.rs"]
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
    let mut result = UnicodeBuffer::new();
    result.push_str(text);
    result.set_language("en".parse().unwrap());
    result.guess_segment_properties();
    result.set_direction(direction);
    result
}
fn compare(bytes: &[u8], text: &str, direction: Direction) -> Meter {
    let font = crate::FontRef::new(bytes).unwrap();
    let data = ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let mut meter = Meter::default();
    let actual = face
        .try_shape_with_scratch(input(text, direction), &[], &mut meter)
        .unwrap();
    let native = face.shape(input(text, direction), &[]);
    let mut buffer = harfrust_reference::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_language("en".parse().unwrap());
    buffer.guess_segment_properties();
    buffer.set_direction(if direction == Direction::LeftToRight {
        harfrust_reference::Direction::LeftToRight
    } else {
        harfrust_reference::Direction::RightToLeft
    });
    let reference = harfrust_reference::ShaperData::new(&font);
    let expected = reference.shaper(&font).build().shape(buffer, &[]);
    for (infos, positions) in [
        (
            format!("{:?}", native.glyph_infos()),
            format!("{:?}", native.glyph_positions()),
        ),
        (
            format!("{:?}", expected.glyph_infos()),
            format!("{:?}", expected.glyph_positions()),
        ),
    ] {
        assert_eq!(
            format!("{:?}", actual.glyph_infos()),
            infos,
            "{direction:?} {text:?}"
        );
        assert_eq!(
            format!("{:?}", actual.glyph_positions()),
            positions,
            "{direction:?} {text:?}"
        );
    }
    meter
}
#[test]
fn match_scratch_inline_spill_geometric_growth_and_every_refusal_preserve_logical_values() {
    let run = |budget: &mut Meter, values: &mut smallvec::SmallVec<[usize; 4]>, target| {
        let mut observer = ObservePlan(budget);
        let mut scratch = PlanScratch::new(Some(&mut observer));
        let result = scratch.resize_match_positions(values, target);
        if let Err(error) = result {
            assert_eq!(scratch.resize_match_positions(values, target), Err(error));
        }
        result
    };
    let mut values = smallvec::SmallVec::from_elem(7, 4);
    let mut native = values.clone();
    let mut full = Meter::default();
    for target in [4, 5, 6, 17, 64] {
        run(&mut full, &mut values, target).unwrap();
        PlanScratch::new(None)
            .resize_match_positions(&mut native, target)
            .unwrap();
        assert_eq!(values, native);
        assert_eq!(values.capacity(), native.capacity());
    }
    assert_eq!(
        full.requests.iter().map(|r| r.count).collect::<Vec<_>>(),
        [8, 32, 64]
    );
    assert!(full.requests.iter().all(|r| r.site == Site::MatchPositions));
    let mut cuts = 0;
    for (start, target) in [(0, 4), (4, 5), (5, 6), (6, 17), (17, 64)] {
        let mut original = smallvec::SmallVec::from_elem(7, start);
        original.shrink_to_fit();
        let mut full = Meter::default();
        let mut actual = original.clone();
        run(&mut full, &mut actual, target).unwrap();
        for cap in 0..full.work {
            let mut meter = Meter {
                maximum_work: Some(cap),
                ..Meter::default()
            };
            let mut values = original.clone();
            assert_eq!(run(&mut meter, &mut values, target), Err(Error::Refused));
            assert_eq!(values, original);
            cuts += 1;
        }
        for index in 0..full.requests.len() {
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            let mut values = original.clone();
            assert_eq!(
                run(&mut meter, &mut values, target),
                Err(Error::AllocationFailure)
            );
            assert_eq!(values, original);
        }
    }
    let mut meter = Meter::default();
    let mut values = smallvec::SmallVec::from_elem(7, 4);
    assert_eq!(
        run(&mut meter, &mut values, usize::MAX),
        Err(Error::ArithmeticOverflow)
    );
    assert_eq!(values.as_slice(), &[7; 4]);
    assert!(meter.requests.is_empty());
    assert_eq!(meter.work, 0);
    std::eprintln!(
        "match SmallVec: growth targets=[8,32,64], inline allocations=0, work cutoffs={cuts}"
    );
}
#[test]
fn match_scratch_all_six_context_formats_gsub_gpos_marks_rtl_match_immutable_registry() {
    let mut comparisons = 0;
    for format in 1..=3 {
        for chained in [false, true] {
            for positioning in [false, true] {
                for marks in [false, true] {
                    let mut config = fixture::Context::sequence(format, chained, &[18; 5], 8);
                    config.positioning = positioning;
                    config.ignore_marks = marks;
                    let bytes = crate::test_font::with_replaced_tables(
                        BASE,
                        &fixture::contextual_tables(&config),
                    );
                    let text = match (chained, marks) {
                        (false, false) => "11111",
                        (false, true) => "1a1111",
                        (true, false) => "0111114",
                        (true, true) => "01a11114",
                    };
                    for direction in [Direction::LeftToRight, Direction::RightToLeft] {
                        for candidate in [text, "1111", "0112114"] {
                            let meter = compare(&bytes, candidate, direction);
                            if candidate == text && direction == Direction::LeftToRight {
                                assert!(meter
                                    .requests
                                    .iter()
                                    .any(|r| r.site == Site::MatchPositions));
                                let font = crate::FontRef::new(&bytes).unwrap();
                                let data = ShaperData::new(&font);
                                let result = data
                                    .shaper(&font)
                                    .build()
                                    .shape(input(text, direction), &[]);
                                if positioning {
                                    assert!(result
                                        .glyph_positions()
                                        .iter()
                                        .any(|p| p.x_advance > 350));
                                } else {
                                    assert!(result.glyph_infos().iter().any(|g| g.glyph_id == 19));
                                    assert!(result.glyph_infos().iter().any(|g| g.glyph_id == 20));
                                }
                            }
                            comparisons += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 144);
    std::eprintln!("match contextual immutable-registry comparisons={comparisons}");
}
fn check_faults(bytes: &[u8], text: &str) -> (usize, usize) {
    let font = crate::FontRef::new(bytes).unwrap();
    let data = ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let mut full = Meter::default();
    face.try_shape_with_scratch(input(text, Direction::LeftToRight), &[], &mut full)
        .unwrap();
    let mut faults = 0;
    for (index, request) in full.requests.iter().enumerate() {
        if request.site != Site::MatchPositions {
            continue;
        }
        let mut meter = Meter {
            fail_at: Some(index),
            ..Meter::default()
        };
        assert!(matches!(
            face.try_shape_with_scratch(input(text, Direction::LeftToRight), &[], &mut meter),
            Err(Error::AllocationFailure)
        ));
        assert_eq!(meter.requests, full.requests[..=index]);
        assert_eq!([meter.spool, meter.work], [request.spool, request.work]);
        faults += 1;
    }
    for cap in 0..full.work {
        let mut meter = Meter {
            maximum_work: Some(cap),
            ..Meter::default()
        };
        assert!(
            matches!(
                face.try_shape_with_scratch(input(text, Direction::LeftToRight), &[], &mut meter),
                Err(Error::Refused)
            ),
            "cap={cap}"
        );
        assert!(meter.work <= cap);
    }
    let mut exact = Meter {
        maximum_work: Some(full.work),
        ..Meter::default()
    };
    face.try_shape_with_scratch(input(text, Direction::LeftToRight), &[], &mut exact)
        .unwrap();
    assert_eq!(exact.requests, full.requests);
    (faults, full.work)
}
#[test]
fn match_scratch_every_context_reservation_and_work_cutoff_includes_nested_growth() {
    let mut faults = 0;
    let mut cuts = 0;
    for format in 1..=3 {
        for chained in [false, true] {
            for positioning in [false, true] {
                let mut config = fixture::Context::sequence(format, chained, &[18; 5], 8);
                config.positioning = positioning;
                let bytes = crate::test_font::with_replaced_tables(
                    BASE,
                    &fixture::contextual_tables(&config),
                );
                let (f, c) = check_faults(&bytes, if chained { "0111114" } else { "11111" });
                assert_eq!(f, if positioning { 1 } else { 2 });
                faults += f;
                cuts += c;
            }
        }
    }
    assert_eq!(faults, 18);
    std::eprintln!("match actual context: paid allocation faults={faults}, work cutoffs={cuts}");
}
#[test]
fn match_scratch_ligature_inline_boundary_ignored_marks_and_overlong_skip_match_registry() {
    let mut comparisons = 0;
    let mut faults = 0;
    let mut cuts = 0;
    for count in [3, 4, 5, 64, 65] {
        for marks in [false, true] {
            let bytes = crate::test_font::with_replaced_tables(
                BASE,
                &fixture::ligature_tables(count, marks),
            );
            let text = if marks {
                format!("1a{}", "1".repeat(count as usize - 1))
            } else {
                "1".repeat(count as usize)
            };
            for direction in [Direction::LeftToRight, Direction::RightToLeft] {
                let meter = compare(&bytes, &text, direction);
                let spills = meter
                    .requests
                    .iter()
                    .filter(|r| r.site == Site::MatchPositions)
                    .count();
                assert_eq!(spills, usize::from(count > 4 && count <= 64));
                comparisons += 1;
            }
            let font = crate::FontRef::new(&bytes).unwrap();
            let data = ShaperData::new(&font);
            let result = data
                .shaper(&font)
                .build()
                .shape(input(&text, Direction::LeftToRight), &[]);
            assert_eq!(
                result
                    .glyph_infos()
                    .iter()
                    .filter(|g| g.glyph_id == 19)
                    .count(),
                usize::from(count <= 64)
            );
            if count == 5 || count == 64 {
                let (f, c) = check_faults(&bytes, &text);
                faults += f;
                cuts += c;
            }
        }
    }
    std::eprintln!("match ligatures: comparisons={comparisons}, paid faults={faults}, work cutoffs={cuts}, 65-component skip retained");
}

#[test]
fn match_scratch_context_boundary_and_malformed_table_keep_original_skip_policy() {
    let mut comparisons = 0;
    for format in 1..=3 {
        for chained in [false, true] {
            for count in [64, 65] {
                let glyphs = vec![18; count];
                let config = fixture::Context::sequence(format, chained, &glyphs, 1);
                let bytes = crate::test_font::with_replaced_tables(
                    BASE,
                    &fixture::contextual_tables(&config),
                );
                let text = if chained {
                    format!("0{}4", "1".repeat(count))
                } else {
                    "1".repeat(count)
                };
                let meter = compare(&bytes, &text, Direction::LeftToRight);
                assert_eq!(
                    meter
                        .requests
                        .iter()
                        .filter(|r| r.site == Site::MatchPositions)
                        .count(),
                    usize::from(count == 64)
                );
                let font = crate::FontRef::new(&bytes).unwrap();
                let data = ShaperData::new(&font);
                let output = data
                    .shaper(&font)
                    .build()
                    .shape(input(&text, Direction::LeftToRight), &[]);
                assert_eq!(
                    output
                        .glyph_infos()
                        .iter()
                        .filter(|g| g.glyph_id == 19)
                        .count(),
                    usize::from(count == 64)
                );
                comparisons += 1;
            }
            let config = fixture::Context::sequence(format, chained, &[18; 5], 8);
            let mut tables = fixture::contextual_tables(&config);
            let table = &mut tables[0].1;
            let at = |bytes: &[u8], offset| {
                u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize
            };
            let list = at(table, 8);
            let lookup = list + at(table, list + 2);
            let subtable = lookup + at(table, lookup + 6);
            table[subtable..subtable + 2].copy_from_slice(&0xffffu16.to_be_bytes());
            let bytes = crate::test_font::with_replaced_tables(BASE, &tables);
            let meter = compare(
                &bytes,
                if chained { "0111114" } else { "11111" },
                Direction::LeftToRight,
            );
            assert!(!meter
                .requests
                .iter()
                .any(|r| r.site == Site::MatchPositions));
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 18);
    std::eprintln!(
        "match contextual boundary/malformed comparisons={comparisons}, 65-input skip retained"
    );
}
