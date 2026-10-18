use super::hb_glyph_info_t;
use crate::indic_test_font as fixture;
use crate::scratch::{Budget, Error, ObservePlan, PlanScratch, Site};
use crate::{Direction, ShaperData, UnicodeBuffer};
use alloc::{format, vec::Vec};
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
        if self.maximum_work.is_some_and(|m| next > m) {
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
    buffer.set_language("hi".parse().unwrap());
    buffer.guess_segment_properties();
    buffer.set_direction(direction);
    buffer
}
fn font(old: bool) -> Vec<u8> {
    crate::test_font::with_replaced_tables(BASE, &fixture::indic_tables(BASE, old))
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
    let reference = harfrust_reference::ShaperData::new(&font);
    let mut buffer = harfrust_reference::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_language("hi".parse().unwrap());
    buffer.guess_segment_properties();
    buffer.set_direction(if direction == Direction::LeftToRight {
        harfrust_reference::Direction::LeftToRight
    } else {
        harfrust_reference::Direction::RightToLeft
    });
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
            "{text:?} {direction:?}"
        );
        assert_eq!(
            format!("{:?}", actual.glyph_positions()),
            positions,
            "{text:?} {direction:?}"
        );
    }
    assert!(
        actual.glyph_infos().iter().all(|g| g.glyph_id != 0),
        "unmapped fixture character: {text:?}"
    );
    meter
}
#[test]
fn indic_scratch_stable_keys_preserve_ties_and_original_values_at_every_refusal() {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Tagged {
        key: u8,
        id: u32,
    }
    let run = |meter: &mut Meter, values: &mut [Tagged]| {
        let mut observer = ObservePlan(meter);
        let mut scratch = PlanScratch::new(Some(&mut observer));
        let result = scratch.stable_sort_by_key(values, |v| v.key, Site::IndicSortRecords);
        if let Err(e) = result {
            assert_eq!(
                scratch.stable_sort_by_key(values, |v| v.key, Site::IndicSortRecords),
                Err(e)
            );
        }
        result
    };
    let mut cuts = 0;
    let mut faults = 0;
    for count in [0, 1, 2, 3, 4, 7, 17, 64, 127, 256] {
        let original = (0..count)
            .map(|i| Tagged {
                key: ((i * 37) % 11) as u8,
                id: i as u32,
            })
            .collect::<Vec<_>>();
        let mut expected = original.clone();
        expected.sort_by_key(|v| v.key);
        let mut full = Meter::default();
        let mut values = original.clone();
        run(&mut full, &mut values).unwrap();
        assert_eq!(values, expected);
        assert_eq!(full.requests.len(), usize::from(count > 1));
        if let Some(request) = full.requests.first() {
            assert_eq!(request.site, Site::IndicSortRecords);
            assert_eq!(request.count, count);
        }
        for cap in 0..full.work {
            let mut meter = Meter {
                maximum_work: Some(cap),
                ..Meter::default()
            };
            let mut values = original.clone();
            assert_eq!(run(&mut meter, &mut values), Err(Error::Refused));
            assert_eq!(values, original);
            cuts += 1;
        }
        for index in 0..full.requests.len() {
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            let mut values = original.clone();
            assert_eq!(run(&mut meter, &mut values), Err(Error::AllocationFailure));
            assert_eq!(values, original);
            faults += 1;
        }
        let mut native = original.clone();
        PlanScratch::new(None)
            .stable_sort_by_key(&mut native, |v| v.key, Site::IndicSortRecords)
            .unwrap();
        assert_eq!(native, expected);
    }
    std::eprintln!("Indic stable keys: paid faults={faults}, work cutoffs={cuts}, tie order and original values retained");
}
#[test]
fn indic_scratch_actual_glyph_record_size_and_stable_position_ties() {
    let mut values = [hb_glyph_info_t::default(); 4];
    for (i, info) in values.iter_mut().enumerate() {
        info.glyph_id = i as u32 + 10;
        info.set_indic_position((i % 2) as u8);
    }
    let mut meter = Meter::default();
    let mut observer = ObservePlan(&mut meter);
    PlanScratch::new(Some(&mut observer))
        .stable_sort_by_key(&mut values, |v| v.indic_position(), Site::IndicSortRecords)
        .unwrap();
    assert_eq!(values.map(|v| v.glyph_id), [10, 12, 11, 13]);
    assert_eq!(meter.requests.len(), 1);
    assert_eq!(meter.requests[0].count, 4);
    assert!(
        meter.requests[0].bytes
            > core::mem::size_of::<hb_glyph_info_t>() + core::mem::size_of::<usize>()
    );
    std::eprintln!(
        "Indic actual indexed glyph record bytes={}",
        meter.requests[0].bytes
    );
}

#[test]
fn indic_scratch_nine_scripts_old_new_spec_marks_joiners_and_rtl_match_registry() {
    let samples = [
        ["किकि", "र्कि", "क्\u{200d}षि", "\u{093f}"],
        ["কিকি", "র্কি", "ক্\u{200d}ষি", "\u{09bf}"],
        ["ਕਿਕਿ", "ਰ੍ਕਿ", "ਕ੍\u{200d}ਖਿ", "\u{0a3f}"],
        ["કિકિ", "ર્કિ", "ક્\u{200d}ષિ", "\u{0abf}"],
        ["କିକି", "ର୍କି", "କ୍\u{200d}ଷି", "\u{0b3f}"],
        ["கெகெ", "ர்கெ", "க்\u{200d}கெ", "\u{0bc6}"],
        ["కెకె", "ర్కె", "క్\u{200d}షె", "\u{0c46}"],
        ["ಕಿಕಿ", "ರ್ಕಿ", "ಕ್\u{200d}ಷಿ", "\u{0cbf}"],
        ["കെകെ", "ര്‍കെ", "ക്\u{200d}ഷെ", "\u{0d46}"],
    ];
    let mut cases = 0;
    let mut allocations = 0;
    for old in [false, true] {
        let bytes = font(old);
        for script in samples {
            for text in script {
                for direction in [Direction::LeftToRight, Direction::RightToLeft] {
                    let meter = compare(&bytes, text, direction);
                    let requests = meter
                        .requests
                        .iter()
                        .filter(|r| r.site == Site::IndicSortRecords)
                        .count();
                    assert!(requests > 0, "old={old} {text:?} {direction:?}");
                    allocations += requests;
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 144);
    std::eprintln!("Indic immutable-registry derived-font cases={cases}, actual sort allocations={allocations}");
}
#[test]
fn indic_scratch_every_actual_sort_reservation_and_work_cutoff_stops_before_publication() {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut faults = 0;
        let mut cuts = 0;
        for old in [false, true] {
            let bytes = font(old);
            let font = crate::FontRef::new(&bytes).unwrap();
            let data = ShaperData::new(&font);
            let face = data.shaper(&font).build();
            for text in ["किकि", "र्कि", "क्\u{200d}षि", "\u{093f}"] {
                let mut full = Meter::default();
                face.try_shape_with_scratch(input(text, Direction::LeftToRight), &[], &mut full)
                    .unwrap();
                std::eprintln!(
                    "Indic cutoff start: old={old}, text={text:?}, work={}, requests={}",
                    full.work,
                    full.requests.len()
                );
                for (index, request) in full.requests.iter().enumerate() {
                    if request.site != Site::IndicSortRecords {
                        continue;
                    }
                    std::eprintln!(
                        "Indic reservation cutoff: old={old}, text={text:?}, index={index}"
                    );
                    let mut meter = Meter {
                        fail_at: Some(index),
                        ..Meter::default()
                    };
                    assert!(matches!(
                        face.try_shape_with_scratch(
                            input(text, Direction::LeftToRight),
                            &[],
                            &mut meter
                        ),
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
                            face.try_shape_with_scratch(
                                input(text, Direction::LeftToRight),
                                &[],
                                &mut meter
                            ),
                            Err(Error::Refused)
                        ),
                        "old={old} {text:?} cap={cap}"
                    );
                    assert!(meter.work <= cap);
                    cuts += 1;
                }
                let mut exact = Meter {
                    maximum_work: Some(full.work),
                    ..Meter::default()
                };
                face.try_shape_with_scratch(input(text, Direction::LeftToRight), &[], &mut exact)
                    .unwrap();
                assert_eq!(exact.requests, full.requests);
            }
        }
        sender.send((faults, cuts)).unwrap();
    });
    let (faults, cuts) = receiver
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("every actual Indic refusal must terminate");
    std::eprintln!("Indic actual sorts: paid allocation faults={faults}, work cutoffs={cuts}");
}
#[test]
fn indic_scratch_long_syllable_sort_and_cluster_boundaries_match_registry() {
    let mut cases = 0;
    for old in [false, true] {
        let bytes = font(old);
        for count in [8, 20, 63, 64, 127] {
            let text = format!("{}कि", "क्".repeat(count));
            let meter = compare(&bytes, &text, Direction::LeftToRight);
            assert!(meter
                .requests
                .iter()
                .any(|r| r.site == Site::IndicSortRecords));
            cases += 1;
        }
    }
    std::eprintln!("Indic long-syllable registry cases={cases}, maximum Unicode characters=256");
}

#[test]
fn indic_scratch_broken_cluster_growth_refusal_returns_before_timeout() {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let bytes = font(false);
        let font = crate::FontRef::new(&bytes).unwrap();
        let data = ShaperData::new(&font);
        let face = data.shaper(&font).build();
        let mut full = Meter::default();
        face.try_shape_with_scratch(input("ि ि ि ि", Direction::LeftToRight), &[], &mut full)
            .unwrap();
        let mut faults = 0;
        for (index, request) in full.requests.iter().enumerate() {
            if !matches!(
                request.site,
                Site::GlyphInfos | Site::GlyphPositions | Site::IndicSortRecords
            ) {
                continue;
            }
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            assert!(matches!(
                face.try_shape_with_scratch(
                    input("ि ि ि ि", Direction::LeftToRight),
                    &[],
                    &mut meter
                ),
                Err(Error::AllocationFailure)
            ));
            assert_eq!(meter.requests, full.requests[..=index]);
            faults += 1;
        }
        assert!(full.requests.iter().any(|r| r.site == Site::GlyphInfos));
        assert!(full.requests.iter().any(|r| r.site == Site::GlyphPositions));
        for cap in 0..full.work {
            let mut meter = Meter {
                maximum_work: Some(cap),
                ..Meter::default()
            };
            assert!(matches!(
                face.try_shape_with_scratch(
                    input("ि ि ि ि", Direction::LeftToRight),
                    &[],
                    &mut meter
                ),
                Err(Error::Refused)
            ));
        }
        sender.send((faults, full.work)).unwrap();
    });
    let (faults, cuts) = receiver
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect(
            "observed broken-cluster refusal must return, even when dotted-circle growth fails",
        );
    assert!(faults >= 3);
    std::eprintln!("Indic bounded broken-cluster allocation faults={faults}, work cutoffs={cuts}");
}

#[test]
fn indic_scratch_oversized_syllable_reports_input_limit_and_keeps_earlier_refusal() {
    for old in [false, true] {
        let bytes = font(old);
        let font = crate::FontRef::new(&bytes).unwrap();
        let data = ShaperData::new(&font);
        let face = data.shaper(&font).build();
        let text = format!("{}कि", "क्".repeat(128));
        assert_eq!(text.chars().count(), 258);
        let mut full = Meter::default();
        assert!(matches!(
            face.try_shape_with_scratch(input(&text, Direction::LeftToRight), &[], &mut full),
            Err(Error::InputLimit)
        ));
        assert!(!full
            .requests
            .iter()
            .any(|r| r.site == Site::IndicSortRecords));
        let mut refused = Meter {
            maximum_work: Some(0),
            ..Meter::default()
        };
        assert!(matches!(
            face.try_shape_with_scratch(input(&text, Direction::LeftToRight), &[], &mut refused),
            Err(Error::Refused)
        ));
        assert!(refused.requests.is_empty());
    }
}
