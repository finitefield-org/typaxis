use super::*;
use crate::indic_test_font as fixture;
use crate::scratch::{Budget, Error, ObservePlan, PlanScratch, Site};
use alloc::{format, vec, vec::Vec};
const BASE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));

#[derive(Default)]
struct Meter {
    events: Vec<usize>,
    work: usize,
    maximum: Option<usize>,
    refused: bool,
    reservations: usize,
}
impl Budget for Meter {
    fn reserve(&mut self, _: Site, count: usize, _: usize) -> Result<(), Error> {
        self.work(count)?;
        self.reservations += 1;
        Ok(())
    }
    fn work(&mut self, amount: usize) -> Result<(), Error> {
        assert!(!self.refused, "observer called after refusal");
        let next = self.work.checked_add(amount).unwrap();
        if self.maximum.is_some_and(|maximum| next > maximum) {
            self.refused = true;
            return Err(Error::Refused);
        }
        self.events.push(amount);
        self.work = next;
        Ok(())
    }
}
fn prefix(events: &[usize], maximum: usize) -> (usize, usize) {
    let mut total = 0;
    for (index, &amount) in events.iter().enumerate() {
        if total + amount > maximum {
            return (index, total);
        }
        total += amount;
    }
    (events.len(), total)
}
fn raw(keys: &[u32], clusters: &[u32], level: u32, output: bool) -> OwnedBuffer {
    let mut value = OwnedBuffer::new();
    value.cluster_level = level;
    value.info = keys
        .iter()
        .zip(clusters)
        .enumerate()
        .map(|(i, (&key, &cluster))| hb_glyph_info_t {
            glyph_id: key,
            cluster,
            mask: (i as u32) << 16,
            ..Default::default()
        })
        .collect();
    value.len = keys.len();
    if output {
        value.have_output = true;
        value.idx = 1;
        value.out_len = 1;
    }
    value
}
fn facts(value: &OwnedBuffer) -> Vec<[u32; 3]> {
    value.info[..value.len]
        .iter()
        .map(|v| [v.glyph_id, v.cluster, v.mask])
        .collect()
}
fn compare(a: &hb_glyph_info_t, b: &hb_glyph_info_t) -> bool {
    a.glyph_id > b.glyph_id
}
fn run_raw(
    keys: &[u32],
    clusters: &[u32],
    level: u32,
    output: bool,
    start: usize,
    end: usize,
    meter: &mut Meter,
) -> OwnedBuffer {
    let mut observer = ObservePlan(meter);
    let mut buffer =
        raw(keys, clusters, level, output).into_working(PlanScratch::new(Some(&mut observer)));
    buffer.sort(start, end, compare);
    if buffer.scratch.status().is_err() {
        assert!(!buffer.successful);
        let before = format!("{:?}", buffer.info);
        buffer.sort(start, end, compare);
        assert_eq!(format!("{:?}", buffer.info), before);
        assert_eq!(buffer.scratch.status(), Err(Error::Refused));
    }
    buffer.into_owned()
}

#[test]
fn buffer_sort_scratch_all_cluster_levels_keep_stable_order_and_every_work_prefix() {
    let cases: &[(&[u32], &[u32], usize, usize, bool)] = &[
        (&[], &[], 0, 0, false),
        (&[1], &[0], 0, 1, false),
        (&[1, 2, 3, 4], &[0, 1, 2, 3], 0, 4, false),
        (&[4, 3, 2, 1], &[0, 1, 2, 3], 0, 4, false),
        (&[3, 1, 2, 1, 2, 1], &[0, 1, 2, 3, 4, 5], 0, 6, false),
        (&[9, 4, 3, 1, 8, 7], &[0, 1, 1, 2, 2, 3], 1, 4, false),
        (&[9, 4, 1, 8], &[2, 2, 1, 3], 1, 3, true),
    ];
    let mut cuts = 0;
    let mut reports = Vec::new();
    for level in 0..=3 {
        for (index, &(keys, clusters, start, end, output)) in cases.iter().enumerate() {
            let mut native =
                raw(keys, clusters, level, output).into_working(PlanScratch::new(None));
            native.sort(start, end, compare);
            let expected = native.into_owned();
            let mut full = Meter::default();
            let actual = run_raw(keys, clusters, level, output, start, end, &mut full);
            assert!(actual.successful);
            assert_eq!(facts(&actual), facts(&expected));
            let mut stable = keys.iter().copied().enumerate().collect::<Vec<_>>();
            stable[start..end].sort_by_key(|v| v.1);
            assert_eq!(
                actual.info[..actual.len]
                    .iter()
                    .map(|v| (v.mask >> 16, v.glyph_id))
                    .collect::<Vec<_>>(),
                stable
                    .iter()
                    .map(|&(i, key)| (i as u32, key))
                    .collect::<Vec<_>>()
            );
            assert_eq!(full.reservations, 0, "insertion sort allocated scratch");
            for maximum in 0..full.work {
                let mut meter = Meter {
                    maximum: Some(maximum),
                    ..Default::default()
                };
                let refused = run_raw(keys, clusters, level, output, start, end, &mut meter);
                assert!(!refused.successful);
                let (events, paid) = prefix(&full.events, maximum);
                assert_eq!(meter.events, full.events[..events]);
                assert_eq!(meter.work, paid);
                assert!(meter.refused);
                cuts += 1;
            }
            let mut exact = Meter {
                maximum: Some(full.work),
                ..Default::default()
            };
            assert_eq!(
                facts(&run_raw(
                    keys, clusters, level, output, start, end, &mut exact
                )),
                facts(&actual)
            );
            reports.push(format!(
                "{{\"case\":{index},\"level\":{level},\"start\":{start},\"end\":{end},\"output\":{output},\"keys\":{keys:?},\"clusters\":{clusters:?},\"final\":{:?},\"work_events\":{:?},\"work\":{}}}",
                facts(&actual), full.events, full.work));
        }
    }
    if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
        let path = std::path::PathBuf::from(directory).join("buffer-sort");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("raw.json"),
            format!(
            "{{\"schema\":\"typaxis.harfrust-buffer-sort/1\",\"cases\":[{}],\"cutoffs\":{cuts}}}\n",
            reports.join(",")),
        )
        .unwrap();
    }
    std::eprintln!(
        "Buffer insertion sort: {} cases, {cuts} work cutoffs, zero scratch reservations",
        reports.len()
    );
}

#[test]
fn buffer_sort_scratch_long_nonmonotone_ranges_and_cached_errors() {
    let keys = (0..256).rev().collect::<Vec<u32>>();
    let clusters = (0..256).collect::<Vec<u32>>();
    for level in [
        HB_BUFFER_CLUSTER_LEVEL_CHARACTERS,
        HB_BUFFER_CLUSTER_LEVEL_GRAPHEMES,
    ] {
        let mut full = Meter::default();
        let output = run_raw(&keys, &clusters, level, false, 0, keys.len(), &mut full);
        assert!(output.successful);
        assert_eq!(output.info[0].glyph_id, 0);
        for maximum in [0, 1, 255, full.work / 2, full.work - 1] {
            let mut meter = Meter {
                maximum: Some(maximum),
                ..Default::default()
            };
            assert!(!run_raw(&keys, &clusters, level, false, 0, keys.len(), &mut meter).successful);
            let (events, paid) = prefix(&full.events, maximum);
            assert_eq!(meter.events, full.events[..events]);
            assert_eq!(meter.work, paid);
        }
        let mut meter = Meter::default();
        let mut observer = ObservePlan(&mut meter);
        let mut buffer =
            raw(&keys, &clusters, level, false).into_working(PlanScratch::new(Some(&mut observer)));
        buffer.scratch.fail(Error::AllocationFailure);
        buffer.sort(0, keys.len(), compare);
        assert_eq!(buffer.scratch.status(), Err(Error::AllocationFailure));
        assert!(!buffer.successful);
        drop(buffer);
        assert!(meter.events.is_empty());
    }
}

fn input(text: &str, direction: Direction, level: crate::BufferClusterLevel) -> UnicodeBuffer {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    buffer.set_direction(direction);
    buffer.set_cluster_level(level);
    buffer
}
fn font(start: u32, end: u32, tag: [u8; 4]) -> Vec<u8> {
    crate::test_font::with_replaced_tables(BASE, &fixture::script_tables(BASE, start..=end, &[tag]))
}
fn reference(
    bytes: &[u8],
    text: &str,
    direction: Direction,
    level: crate::BufferClusterLevel,
) -> (alloc::string::String, alloc::string::String) {
    let font = crate::FontRef::new(bytes).unwrap();
    let data = harfrust_reference::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let mut buffer = harfrust_reference::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    buffer.set_direction(if direction == Direction::LeftToRight {
        harfrust_reference::Direction::LeftToRight
    } else {
        harfrust_reference::Direction::RightToLeft
    });
    buffer.set_cluster_level(match level {
        crate::BufferClusterLevel::MonotoneGraphemes => {
            harfrust_reference::BufferClusterLevel::MonotoneGraphemes
        }
        crate::BufferClusterLevel::MonotoneCharacters => {
            harfrust_reference::BufferClusterLevel::MonotoneCharacters
        }
        crate::BufferClusterLevel::Characters => harfrust_reference::BufferClusterLevel::Characters,
        crate::BufferClusterLevel::Graphemes => harfrust_reference::BufferClusterLevel::Graphemes,
    });
    let result = face.shape(buffer, &[]);
    (
        format!("{:?}", result.glyph_infos()),
        format!("{:?}", result.glyph_positions()),
    )
}
fn actual_shape(
    bytes: &[u8],
    text: &str,
    direction: Direction,
    level: crate::BufferClusterLevel,
) -> Meter {
    let font = crate::FontRef::new(bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let mut meter = Meter::default();
    let actual = face
        .try_shape_with_scratch(input(text, direction, level), &[], &mut meter)
        .unwrap();
    let native = face.shape(input(text, direction, level), &[]);
    let expected = reference(bytes, text, direction, level);
    assert_eq!(
        (
            format!("{:?}", actual.glyph_infos()),
            format!("{:?}", actual.glyph_positions())
        ),
        expected
    );
    assert_eq!(
        (
            format!("{:?}", native.glyph_infos()),
            format!("{:?}", native.glyph_positions())
        ),
        expected
    );
    meter
}

#[test]
fn buffer_sort_scratch_normalization_and_myanmar_match_registry_all_cluster_levels() {
    let mut cases = 0;
    for (bytes, texts) in [
        (
            font(0x0300, 0x036f, *b"latn"),
            vec![
                "x\u{301}\u{327}",
                "x\u{315}\u{300}\u{323}",
                "x\u{327}\u{301}",
            ],
        ),
        (
            font(0x1000, 0x109f, *b"mym2"),
            vec!["ကိကိ", "ကြေ", "ြေက", "ေ ိ ြ"],
        ),
        (font(0x1000, 0x109f, *b"mymr"), vec!["ကိကိ", "ကြေ", "ြေက"]),
    ] {
        for text in texts {
            for direction in [Direction::LeftToRight, Direction::RightToLeft] {
                for level in [
                    crate::BufferClusterLevel::MonotoneGraphemes,
                    crate::BufferClusterLevel::MonotoneCharacters,
                    crate::BufferClusterLevel::Characters,
                    crate::BufferClusterLevel::Graphemes,
                ] {
                    actual_shape(&bytes, text, direction, level);
                    cases += 1;
                }
            }
        }
    }
    std::eprintln!("Buffer sort: {cases} full-shape registry comparisons");
}

#[test]
fn buffer_sort_scratch_real_shape_refusals_terminate_and_keep_every_work_prefix() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut cuts = 0;
        for (bytes, text) in [
            (font(0x0300, 0x036f, *b"latn"), "x\u{315}\u{300}\u{323}"),
            (font(0x1000, 0x109f, *b"mym2"), "ကြေ ကိ"),
        ] {
            let font = crate::FontRef::new(&bytes).unwrap();
            let data = crate::ShaperData::new(&font);
            let face = data.shaper(&font).build();
            let level = crate::BufferClusterLevel::MonotoneGraphemes;
            let mut full = Meter::default();
            face.try_shape_with_scratch(input(text, Direction::LeftToRight, level), &[], &mut full)
                .unwrap();
            for maximum in 0..full.work {
                let mut meter = Meter {
                    maximum: Some(maximum),
                    ..Default::default()
                };
                assert!(matches!(
                    face.try_shape_with_scratch(
                        input(text, Direction::LeftToRight, level),
                        &[],
                        &mut meter
                    ),
                    Err(Error::Refused)
                ));
                let (events, paid) = prefix(&full.events, maximum);
                assert_eq!(meter.events, full.events[..events]);
                assert_eq!(meter.work, paid);
                cuts += 1;
            }
            let mut exact = Meter {
                maximum: Some(full.work),
                ..Default::default()
            };
            assert!(face
                .try_shape_with_scratch(input(text, Direction::LeftToRight, level), &[], &mut exact)
                .is_ok());
        }
        sender.send(cuts).unwrap();
    });
    let cuts = receiver
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("work refusal did not terminate");
    worker.join().unwrap();
    std::eprintln!("Buffer sort actual shaping: {cuts} work cutoffs, no partial successful result");
}
