use super::*;
use crate::scratch::{Budget, Error, Site};
use alloc::{format, string::String, vec, vec::Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Request {
    site: Site,
    count: usize,
    element_bytes: usize,
    spool: usize,
    work: usize,
}

#[derive(Default)]
struct Meter {
    spool: usize,
    work: usize,
    requests: Vec<Request>,
    fail_at: Option<usize>,
    maximum_work: Option<usize>,
}

impl Budget for Meter {
    fn reserve(&mut self, site: Site, count: usize, element_bytes: usize) -> Result<(), Error> {
        let bytes = count.checked_mul(element_bytes).unwrap();
        self.work(bytes)?;
        self.spool += bytes;
        self.requests.push(Request {
            site,
            count,
            element_bytes,
            spool: self.spool,
            work: self.work,
        });
        if self.fail_at == Some(self.requests.len() - 1) {
            return Err(Error::AllocationFailure);
        }
        Ok(())
    }
    fn work(&mut self, count: usize) -> Result<(), Error> {
        let next = self.work.checked_add(count).unwrap();
        if self.maximum_work.is_some_and(|maximum| next > maximum) {
            return Err(Error::Refused);
        }
        self.work = next;
        Ok(())
    }
}

fn u16s(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

fn coverage(ranges: bool) -> Vec<u8> {
    if ranges {
        u16s(&[2, 1, 2, 3, 0])
    } else {
        u16s(&[1, 2, 2, 3])
    }
}

fn layout_table(ranges: bool, bad_subtable: bool) -> Vec<u8> {
    // GSUB/GPOS 1.0, one lookup, one single-substitution/positioning subtable.
    // The final word of its header is a delta (GSUB) or value-format (GPOS).
    // Zero is valid for both, so one immutable byte sequence tests both hosts.
    let mut bytes = u16s(&[1, 0, 0, 0, 10, 1, 4, 1, 0, 1]);
    bytes.extend(u16s(&[if bad_subtable { u16::MAX } else { 8 }]));
    bytes.extend(u16s(&[1, 6, 0]));
    bytes.extend(coverage(ranges));
    bytes
}

fn mark_table(ranges: bool) -> Vec<u8> {
    // GDEF 1.2, a MarkGlyphSetsDef table and one 32-bit coverage offset.
    let mut bytes = u16s(&[1, 2, 0, 0, 0, 0, 14, 1, 1, 0, 8]);
    bytes.extend(coverage(ranges));
    bytes
}

fn font(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut bytes = vec![0, 1, 0, 0];
    bytes.extend(u16s(&[tables.len() as u16, 0, 0, 0]));
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in tables {
        bytes.extend(*tag);
        bytes.extend(0u32.to_be_bytes());
        bytes.extend((offset as u32).to_be_bytes());
        bytes.extend((data.len() as u32).to_be_bytes());
        offset += (data.len() + 3) & !3;
    }
    for (_, data) in tables {
        bytes.extend(data);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
    }
    bytes
}

fn test_font(ranges: bool, bad_subtable: bool) -> Vec<u8> {
    font(&[
        (b"GSUB", layout_table(ranges, bad_subtable)),
        (b"GPOS", layout_table(ranges, false)),
        (b"GDEF", mark_table(ranges)),
    ])
}

fn cache_snapshot(data: &ShaperData) -> String {
    use super::super::ot::lookup::LookupState;
    let mut rows = Vec::new();
    for cache in [&data.ot_cache.gsub, &data.ot_cache.gpos] {
        let lookups = cache
            .lookups
            .iter()
            .map(|l| {
                let state = match l.state {
                    LookupState::Vacant => 0,
                    LookupState::Error => 1,
                    LookupState::Ready => 2,
                };
                format!(
                    "{state}/{}/{}/{}/{}/{}/{}/{:?}/{:?}",
                    l.props,
                    l.is_subst,
                    l.is_rtl,
                    l.is_reversed,
                    l.subtables_start,
                    l.subtables_count,
                    l.digest,
                    l.subtable_cache_user_idx
                )
            })
            .collect::<Vec<_>>();
        let subtables = cache
            .subtables
            .iter()
            .map(|s| {
                format!(
                    "{}/{}/{}/{}/{:?}",
                    s.offset, s.coverage_offset, s.is_subst, s.lookup_type, s.digest
                )
            })
            .collect::<Vec<_>>();
        rows.push((lookups, subtables));
    }
    format!("{rows:?}/{:?}", data.ot_cache.gdef_mark_set_digests)
}

#[test]
fn cache_scratch_faults_every_paid_allocation_and_work_prefix() {
    let mut faults = 0;
    let mut work_cuts = 0;
    for ranges in [false, true] {
        let bytes = test_font(ranges, false);
        let face = FontRef::new(&bytes).unwrap();
        let mut full = Meter::default();
        let actual = ShaperData::try_new_with_scratch(&face, &mut full).unwrap();
        assert_eq!(
            cache_snapshot(&actual),
            cache_snapshot(&ShaperData::new(&face))
        );
        assert_eq!(actual.ot_cache.gsub.lookups.len(), 1);
        assert_eq!(actual.ot_cache.gsub.subtables.len(), 1);
        assert_eq!(actual.ot_cache.gpos.subtables.len(), 1);
        assert_eq!(actual.ot_cache.gdef_mark_set_digests.len(), 1);
        assert_eq!(
            full.requests.iter().map(|r| r.site).collect::<Vec<_>>(),
            vec![
                Site::GsubLookups,
                Site::GsubSubtables,
                Site::GposLookups,
                Site::GposSubtables,
                Site::GdefMarkSets
            ]
        );
        for index in 0..full.requests.len() {
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            assert_eq!(
                ShaperData::try_new_with_scratch(&face, &mut meter)
                    .err()
                    .unwrap(),
                Error::AllocationFailure
            );
            assert_eq!(meter.requests, full.requests[..=index]);
            assert_eq!(
                (meter.spool, meter.work),
                (full.requests[index].spool, full.requests[index].work)
            );
            faults += 1;
        }
        for maximum in 0..full.work {
            let mut meter = Meter {
                maximum_work: Some(maximum),
                ..Meter::default()
            };
            assert_eq!(
                ShaperData::try_new_with_scratch(&face, &mut meter)
                    .err()
                    .unwrap(),
                Error::Refused
            );
            assert!(meter.work <= maximum && meter.spool <= full.spool);
            assert_eq!(meter.requests, full.requests[..meter.requests.len()]);
            work_cuts += 1;
        }
    }
    std::eprintln!(
        "Harfrust cache sites=5, post-payment allocation faults={faults}, work cuts={work_cuts}"
    );
}

#[test]
fn cache_scratch_malformed_font_skips_match_original_and_keep_budget_errors_fatal() {
    for ranges in [false, true] {
        for table_index in 0..3 {
            let tables = [
                (b"GSUB", layout_table(ranges, false)),
                (b"GPOS", layout_table(ranges, false)),
                (b"GDEF", mark_table(ranges)),
            ];
            for cut in 0..=tables[table_index].1.len() {
                let mut variant = tables.clone();
                variant[table_index].1.truncate(cut);
                let bytes = font(&variant);
                let face = FontRef::new(&bytes).unwrap();
                let mut meter = Meter::default();
                let actual = ShaperData::try_new_with_scratch(&face, &mut meter).unwrap();
                assert_eq!(
                    cache_snapshot(&actual),
                    cache_snapshot(&ShaperData::new(&face))
                );
            }
        }
        let bytes = test_font(ranges, true);
        let face = FontRef::new(&bytes).unwrap();
        let mut full = Meter::default();
        let data = ShaperData::try_new_with_scratch(&face, &mut full).unwrap();
        assert!(data.ot_cache.gsub.subtables.is_empty());
        assert_eq!(data.ot_cache.gpos.subtables.len(), 1);
        for index in 0..full.requests.len() {
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            assert_eq!(
                ShaperData::try_new_with_scratch(&face, &mut meter)
                    .err()
                    .unwrap(),
                Error::AllocationFailure
            );
            assert_eq!(meter.requests, full.requests[..=index]);
        }
    }
}

#[test]
fn cache_scratch_reallocation_charges_whole_target_and_retains_old_requests() {
    let mut values = Vec::new();
    let mut meter = Meter::default();
    for value in 0..9u32 {
        crate::scratch::push(&mut values, value, Site::GsubSubtables, &mut meter).unwrap();
    }
    assert_eq!(values, (0..9).collect::<Vec<_>>());
    assert_eq!(
        meter.requests.iter().map(|r| r.count).collect::<Vec<_>>(),
        vec![4, 8, 16]
    );
    assert_eq!(meter.spool, (4 + 8 + 16) * core::mem::size_of::<u32>());
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARFRUST_CACHE_FONTS list of original font paths"]
fn cache_scratch_original_fonts_match_immutable_registry_shaping() {
    // A newline-separated file avoids adding JSON or filesystem parsing to the library.
    let list = std::env::var_os("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap();
    let paths = std::fs::read_to_string(list).unwrap();
    let mut cases = 0;
    for path in paths.lines().filter(|p| !p.is_empty()) {
        let bytes = std::fs::read(path).unwrap();
        let face = FontRef::new(&bytes).unwrap();
        let reference = harfrust_reference::ShaperData::new(&face);
        let legacy = ShaperData::new(&face);
        let mut meter = Meter::default();
        let actual = ShaperData::try_new_with_scratch(&face, &mut meter).unwrap();
        assert_eq!(cache_snapshot(&actual), cache_snapshot(&legacy));
        for text in [
            "(123)",
            "日本語123",
            "AV ffi",
            "x\u{0301}",
            "العربية 123",
            "אבג (12)",
            "বাংলা",
            "हिन्दी",
            "ภาษาไทย",
            "한글",
            "កម្ពុជា",
            "မြန်မာ",
        ] {
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(text);
            buffer.guess_segment_properties();
            let shaped = actual.shaper(&face).build().shape(buffer, &[]);
            let mut old_buffer = harfrust_reference::UnicodeBuffer::new();
            old_buffer.push_str(text);
            old_buffer.guess_segment_properties();
            let expected = reference.shaper(&face).build().shape(old_buffer, &[]);
            assert_eq!(
                format!("{:?}", shaped.glyph_infos()),
                format!("{:?}", expected.glyph_infos()),
                "{path} {text:?}"
            );
            assert_eq!(
                format!("{:?}", shaped.glyph_positions()),
                format!("{:?}", expected.glyph_positions()),
                "{path} {text:?}"
            );
            cases += 1;
        }
        std::eprintln!(
            "Harfrust original-font cache {path}: requests={}, spool={}, work={}",
            meter.requests.len(),
            meter.spool,
            meter.work
        );
    }
    assert!(cases >= 48);
    std::eprintln!("Harfrust immutable-registry original-font shaping cases={cases}");
}
