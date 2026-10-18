use super::*;
use crate::shared_shape_storage::Site;
use read_fonts::{FontData, FontRead};

const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));
const COLLECTION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/collection.bin"));

#[derive(Default)]
struct Trace {
    calls: u64,
    work: u64,
    stop: Option<u64>,
}
impl Storage for Trace {
    fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
        panic!("coverage must not allocate")
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("coverage must not retain output")
    }
    fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
        let call = self.calls;
        self.calls += 1;
        if self.stop == Some(call) {
            return Err(LinkedShaperError::AllocationFailure);
        }
        self.work += n;
        Ok(())
    }
}
fn u16s(b: &mut Vec<u8>, values: impl IntoIterator<Item = u16>) {
    for v in values {
        b.extend(v.to_be_bytes());
    }
}
fn u32s(b: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        b.extend(v.to_be_bytes());
    }
}
fn format0_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    u16s(&mut b, [0, 262, 0]);
    b.extend((0..256).map(|cp| (cp % 23) as u8));
    b
}
fn format6_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    u16s(&mut b, [6, 20, 0, 0x40, 5, 0, 4, 7, 0, 12]);
    b
}
fn format4_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    u16s(&mut b, [4, 54, 0, 8, 8, 2, 0]);
    u16s(&mut b, [0x21, 0x43, 0xfffd, 0xffff, 0]);
    u16s(&mut b, [0x20, 0x41, 0xfffd, 0xffff]);
    u16s(&mut b, [5, (-1i16) as u16, 3, 1]);
    u16s(&mut b, [0, 6, 0, 0, 2, 0, 0xffff]);
    b
}
fn group_bytes(constant: bool) -> Vec<u8> {
    let groups = [
        (0, 0, 0),
        (0x40, 0x43, 11),
        (0x10000, 0x1000f, 0xffff_fffc),
        (0x1f600, 0x1f603, 35),
    ];
    let mut b = Vec::new();
    u16s(&mut b, [if constant { 13 } else { 12 }, 0]);
    u32s(
        &mut b,
        [16 + 12 * groups.len() as u32, 0, groups.len() as u32],
    );
    for (a, z, g) in groups {
        u32s(&mut b, [a, z, g]);
    }
    b
}
fn cmap_bytes(tables: &[Vec<u8>], broken_first: bool) -> Vec<u8> {
    let count = tables.len() + usize::from(broken_first);
    let mut b = Vec::new();
    u16s(&mut b, [0, count as u16]);
    let mut offset = 4 + 8 * count;
    if broken_first {
        u16s(&mut b, [0, 0]);
        u32s(&mut b, [u32::MAX]);
    }
    for (index, t) in tables.iter().enumerate() {
        u16s(&mut b, [0, index as u16]);
        u32s(&mut b, [offset as u32]);
        offset += t.len();
    }
    for t in tables {
        b.extend(t);
    }
    b
}
#[test]
fn truetype_coverage_all_nominal_formats_match_pinned_reader_and_stop_at_every_visit() {
    let f0 = format0_bytes();
    let f4 = format4_bytes();
    let f6 = format6_bytes();
    let f12 = group_bytes(false);
    let f13 = group_bytes(true);
    let unsupported = vec![0, 14, 0, 0, 0, 10, 0, 0, 0, 0];
    let variants = vec![
        vec![f0.clone()],
        vec![f4.clone()],
        vec![f6.clone()],
        vec![f12.clone()],
        vec![f13.clone()],
        vec![unsupported.clone(), f4.clone(), f12.clone()],
        vec![f0.clone(), f12.clone()],
        vec![f4.clone(), f12.clone()],
        vec![f12.clone(), f4.clone()],
        vec![unsupported],
        vec![f6, f13],
    ];
    let mut cases = 0;
    let mut cutoffs = 0;
    for tables in variants {
        for broken in [false, true] {
            let bytes = cmap_bytes(&tables, broken);
            let cmap = Cmap::read(FontData::new(&bytes)).unwrap();
            for cp in (0..300)
                .chain(0xfffc..0x10020)
                .chain(0x1f5ff..0x1f609)
                .chain([0x10ffff, u32::MAX])
            {
                let expected = cmap.map_codepoint(cp).map(|g| g.to_u32());
                let mut full = Trace::default();
                assert_eq!(
                    map_codepoint(&cmap, cp, &mut full).unwrap(),
                    expected,
                    "cp={cp:x}"
                );
                for stop in 0..full.calls {
                    let mut short = Trace {
                        stop: Some(stop),
                        ..Trace::default()
                    };
                    assert_eq!(
                        map_codepoint(&cmap, cp, &mut short),
                        Err(E::Backend(LinkedShaperError::AllocationFailure))
                    );
                    assert_eq!(short.calls, stop + 1);
                    cutoffs += 1;
                }
                cases += 1;
            }
        }
    }
    let first_zero = cmap_bytes(&[format0_bytes(), group_bytes(false)], false);
    let cmap = Cmap::read(FontData::new(&first_zero)).unwrap();
    assert_eq!(
        map_codepoint(&cmap, 0, &mut Trace::default()).unwrap(),
        Some(0)
    );
    let ranged = cmap_bytes(&[format4_bytes(), group_bytes(false)], false);
    let cmap = Cmap::read(FontData::new(&ranged)).unwrap();
    assert_eq!(
        map_codepoint(&cmap, 0x41, &mut Trace::default()).unwrap(),
        Some(1)
    );
    assert_eq!(
        map_codepoint(&cmap, 0x42, &mut Trace::default()).unwrap(),
        Some(13)
    );
    assert_eq!(
        map_codepoint(&cmap, 0xfffd, &mut Trace::default()).unwrap(),
        Some(0)
    );
    eprintln!("TrueType nominal cases={cases}, refusal cutoffs={cutoffs}");
}

#[test]
fn truetype_coverage_directory_setup_matches_single_ttc_and_unsorted_faces() {
    let mut unsorted = FONT.to_vec();
    let a = unsorted[12..28].to_vec();
    let b = unsorted[28..44].to_vec();
    unsorted[12..28].copy_from_slice(&b);
    unsorted[28..44].copy_from_slice(&a);
    // The retained collection has one face. Add a second index sharing its
    // directory and adjust the absolute table offsets after the larger header.
    let mut two = COLLECTION.to_vec();
    two[8..12].copy_from_slice(&2u32.to_be_bytes());
    let directory = u32::from_be_bytes(two[12..16].try_into().unwrap()) + 4;
    two[12..16].copy_from_slice(&directory.to_be_bytes());
    two.splice(16..16, directory.to_be_bytes());
    let count = u16::from_be_bytes(
        two[directory as usize + 4..directory as usize + 6]
            .try_into()
            .unwrap(),
    );
    for i in 0..usize::from(count) {
        let at = directory as usize + 12 + i * 16 + 8;
        let offset = u32::from_be_bytes(two[at..at + 4].try_into().unwrap()) + 4;
        two[at..at + 4].copy_from_slice(&offset.to_be_bytes());
    }
    for (bytes, index) in [
        (FONT, 0),
        (COLLECTION, 0),
        (two.as_slice(), 1),
        (unsorted.as_slice(), 0),
    ] {
        let face = harfrust::FontRef::from_index(bytes, index).unwrap();
        let expected = face.cmap().unwrap();
        let mut trace = Trace::default();
        let actual = load_cmap(bytes, index, &mut trace).unwrap();
        assert_eq!(trace.calls, 2);
        let n = u64::from(face.table_directory().num_tables());
        assert_eq!(trace.work, 3 * n + 17);
        for cp in [0u32, 32, 65, 82, 0x3042, 0x4e00, 0xffff, 0x1f600] {
            assert_eq!(actual.map_codepoint(cp), expected.map_codepoint(cp));
        }
        for stop in 0..2 {
            let mut refused = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            assert!(matches!(
                load_cmap(bytes, index, &mut refused),
                Err(E::Backend(LinkedShaperError::AllocationFailure))
            ));
            assert_eq!(refused.calls, stop + 1);
        }
    }
    for (bytes, index) in [(&[][..], 0), (FONT, 1), (COLLECTION, u32::MAX)] {
        assert!(matches!(
            load_cmap(bytes, index, &mut Trace::default()),
            Err(E::InvalidFontOrFace)
        ));
    }
}

#[test]
#[ignore = "requires retained fonts in TYPAXIS_HARFRUST_CACHE_FONTS"]
fn truetype_coverage_original_fonts_match_bmp_and_supplementary_group_boundaries() {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let mut all = 0;
    let mut work_total = 0;
    for path in list.lines() {
        let bytes = std::fs::read(path).unwrap();
        let face = harfrust::FontRef::from_index(&bytes, 0).unwrap();
        let expected = face.cmap().unwrap();
        let mut trace = Trace::default();
        let actual = load_cmap(&bytes, 0, &mut trace).unwrap();
        let mut queries = (0..=0xffff).collect::<Vec<u32>>();
        for record in expected.encoding_records() {
            match record.subtable(expected.offset_data()) {
                Ok(CmapSubtable::Format12(v)) => {
                    for g in v.groups() {
                        for cp in [
                            g.start_char_code().saturating_sub(1),
                            g.start_char_code(),
                            g.end_char_code(),
                            g.end_char_code().saturating_add(1),
                        ] {
                            queries.push(cp);
                        }
                    }
                }
                Ok(CmapSubtable::Format13(v)) => {
                    for g in v.groups() {
                        for cp in [g.start_char_code(), g.end_char_code()] {
                            queries.push(cp);
                        }
                    }
                }
                _ => (),
            }
        }
        queries.extend([0x10ffff, u32::MAX]);
        for cp in &queries {
            assert_eq!(
                map_codepoint(&actual, *cp, &mut trace).unwrap(),
                expected.map_codepoint(*cp).map(|g| g.to_u32()),
                "font={path}, cp={cp:x}"
            );
        }
        eprintln!(
            "original nominal font={} cases={} work={}",
            path,
            queries.len(),
            trace.work
        );
        all += queries.len();
        work_total += trace.work;
    }
    eprintln!("original nominal coverage cases={all} work={work_total}");
}
