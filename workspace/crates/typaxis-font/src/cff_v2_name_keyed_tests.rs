use super::*;
use crate::cff::font_tables::tests::cases as sfnt_cases;
const TINY:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/fixture-fonts/typaxis-cff-fixture.otf"));
#[derive(Default)]
struct Trace {
    requests: Vec<(u8, usize, usize)>,
    stop: Option<usize>,
}
impl Trace {
    fn request(&mut self, value: (u8, usize, usize)) -> Result<(), usize> {
        let at = self.requests.len();
        self.requests.push(value);
        if self.stop == Some(at) {
            Err(at)
        } else {
            Ok(())
        }
    }
}
impl CffBaseCmapBudgetV2 for Trace {
    type Error = usize;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        Ok(())
    }
    fn work(&mut self, n: u64) -> Result<(), usize> {
        self.request((65, n as usize, 0))
    }
}
impl CffCmapBudgetV2 for Trace {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
}
impl CffAdmissionBudgetV2 for Trace {
    const OBSERVE_TABLES: bool = true;
    const OBSERVE_PROGRAM: bool = true;
    fn reserve_sfnt(
        &mut self,
        _: CffSfntStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), usize> {
        self.request((0, rows, bytes))
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), usize> {
        self.request((1, n as usize, 0))
    }
    fn work_tables(&mut self, n: u64) -> Result<(), usize> {
        self.request((2, n as usize, 0))
    }
    fn reserve_program(&mut self, site: PSite, rows: usize, bytes: usize) -> Result<(), usize> {
        self.request((3 + site as u8, rows, bytes))
    }
    fn work_program(&mut self, n: u64) -> Result<(), usize> {
        self.request((32, n as usize, 0))
    }
}
fn index(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = (objects.len() as u16).to_be_bytes().to_vec();
    if objects.is_empty() {
        return out;
    }
    out.push(2);
    let mut offset = 1u16;
    out.extend(offset.to_be_bytes());
    for o in objects {
        offset += o.len() as u16;
        out.extend(offset.to_be_bytes());
    }
    for o in objects {
        out.extend(o);
    }
    out
}
fn op(out: &mut Vec<u8>, values: &[usize], op: u8) {
    for value in values {
        out.push(29);
        out.extend((*value as i32).to_be_bytes());
    }
    out.push(op);
}
fn fixture(predefined: usize, charset: Option<&[u8]>, encoding: Option<&[u8]>) -> Vec<u8> {
    let names = index(&[b"TypaxisNameTest".to_vec()]);
    let chars = index(&[vec![14], vec![14], vec![14]]);
    let mut private = Vec::new();
    op(&mut private, &[600], 20);
    let mut positions = [0; 4];
    for _ in 0..3 {
        let mut top = Vec::new();
        op(
            &mut top,
            &[if charset.is_some() {
                positions[0]
            } else {
                predefined
            }],
            15,
        );
        op(
            &mut top,
            &[if encoding.is_some() { positions[1] } else { 0 }],
            16,
        );
        op(&mut top, &[positions[2]], 17);
        op(&mut top, &[private.len(), positions[3]], 18);
        let top = index(&[top]);
        let prefix = 4 + names.len() + top.len() + 4;
        let next = [
            prefix,
            prefix + charset.map_or(0, <[u8]>::len),
            prefix + charset.map_or(0, <[u8]>::len) + encoding.map_or(0, <[u8]>::len),
            prefix + charset.map_or(0, <[u8]>::len) + encoding.map_or(0, <[u8]>::len) + chars.len(),
        ];
        if positions == next {
            let mut out = vec![1, 0, 4, 4];
            out.extend(&names);
            out.extend(&top);
            out.extend([0, 0, 0, 0]);
            if let Some(b) = charset {
                out.extend(b);
            }
            if let Some(b) = encoding {
                out.extend(b);
            }
            out.extend(&chars);
            out.extend(&private);
            return out;
        }
        positions = next;
    }
    panic!("fixture did not converge")
}
fn facts(p: &CffProgramInspectionV2) -> Vec<(Option<u8>, Option<CffSourceGlyphV2>, Vec<u8>)> {
    (0..p.glyph_count())
        .map(|g| {
            (
                p.fd_for_gid(g),
                p.source_glyph(g),
                p.charstring(g).unwrap().to_vec(),
            )
        })
        .collect()
}
#[test]
fn cff_v2_name_keyed_charsets_encodings_and_every_callback_prefix() {
    let mut controls = Vec::new();
    for p in 0..3 {
        controls.push(fixture(p, None, None));
    }
    for charset in [
        &[0, 0, 34, 0, 35][..],
        &[1, 0, 34, 1][..],
        &[2, 0, 34, 0, 1][..],
    ] {
        for encoding in [
            None,
            Some(&[0, 2, 65, 66][..]),
            Some(&[1, 1, 65, 1][..]),
            Some(&[0x80, 1, 65, 1, 66, 0, 35][..]),
        ] {
            controls.push(fixture(0, Some(charset), encoding));
        }
    }
    let (mut cuts, mut truncations) = (0, 0);
    for b in controls.iter() {
        let native = inspect_cff1_program_v2(Arc::from(b.as_slice()), 3, 1000, 20).unwrap();
        assert_eq!(native.font_dict_count(), 1);
        assert_eq!(native.width_defaults(0), Some((600 * 65536, 0)));
        for gid in 0..3 {
            assert_eq!(native.fd_for_gid(gid), Some(0));
            assert_eq!(native.cid_for_gid(gid), None);
            assert!(native.sid_for_gid(gid).is_some());
        }
        let mut full = Trace::default();
        let observed =
            inspect_cff1_program_v2_with_budget(Arc::from(b.as_slice()), 3, 1000, 20, &mut full)
                .unwrap()
                .unwrap();
        assert_eq!(facts(&native), facts(&observed));
        for stop in 0..full.requests.len() {
            let mut short = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let failure = inspect_cff1_program_v2_with_budget(
                Arc::from(b.as_slice()),
                3,
                1000,
                20,
                &mut short,
            )
            .unwrap_err();
            assert_eq!(failure.error, stop);
            assert_eq!(short.requests, full.requests[..=stop]);
            cuts += 1;
        }
        for end in 0..b.len() {
            let expected = inspect_cff1_program_v2(Arc::from(&b[..end]), 3, 1000, 20).unwrap_err();
            let actual = inspect_cff1_program_v2_with_budget(
                Arc::from(&b[..end]),
                3,
                1000,
                20,
                &mut Trace::default(),
            )
            .unwrap()
            .unwrap_err();
            assert_eq!(expected, actual);
            truncations += 1;
        }
    }
    eprintln!(
        "name-keyed positive controls={} all callback cuts={cuts} truncations={truncations}",
        controls.len()
    );
}
#[test]
fn cff_v2_name_keyed_rejects_invalid_sids_codes_and_mixed_cid_operators() {
    for charset in [
        &[0, 0, 34, 0, 34][..],
        &[0, 0, 0, 0, 35][..],
        &[0, 1, 135, 0, 35][..],
        &[1, 0, 34, 2][..],
        &[2, 255, 255, 0, 1][..],
    ] {
        let b = fixture(0, Some(charset), None);
        assert_eq!(
            inspect_cff1_program_v2(b.into(), 3, 1000, 20)
                .unwrap_err()
                .kind,
            K::InvalidCharset
        );
    }
    for encoding in [
        &[0, 2, 65, 65][..],
        &[0, 3, 65, 66, 67][..],
        &[1, 1, 255, 1][..],
        &[1, 2, 65, 1, 66, 0][..],
        &[2, 0][..],
        &[0x80, 1, 65, 1, 65, 0, 35][..],
        &[0x80, 1, 65, 1, 66, 0, 36][..],
        &[0x80, 1, 65, 1, 66, 0, 0][..],
    ] {
        let b = fixture(0, Some(&[0, 0, 34, 0, 35]), Some(encoding));
        assert_eq!(
            inspect_cff1_program_v2(b.into(), 3, 1000, 20)
                .unwrap_err()
                .kind,
            K::InvalidEncoding
        );
    }
    let mut b = fixture(0, None, None);
    // Preserve INDEX sizes and offsets, so rejection must come from the
    // name-keyed Top DICT's CID-only FDSelect, rather than a damaged INDEX.
    let at = b
        .windows(6)
        .position(|v| v == [29, 0, 0, 0, 0, 15])
        .unwrap();
    b[at..at + 6].copy_from_slice(&[139, 12, 37, 139, 139, 0]);
    let error = inspect_cff1_program_v2(b.into(), 3, 1000, 20).unwrap_err();
    assert_eq!(error.kind, K::UnsupportedOperator);
    assert_eq!(error.operator, Some(0x0c25));
    assert_eq!(error.table_offset, at + 1);
}
#[test]
fn cff_v2_name_keyed_fftm_is_versioned_validated_and_observed() {
    let caps = limits();
    let mut metadata = vec![0; 28];
    metadata[3] = 1;
    let source = sfnt_cases::replace(TINY, *b"FFTM", Some(metadata.clone()));
    let old = admit_sfnt_cff1_detailed(&source, 0, &caps).unwrap_err();
    assert_eq!(old.kind, Cff1Error::UnsupportedTable);
    assert_eq!(old.context.table_tag, Some(*b"FFTM"));
    let native = admit_sfnt_cff1_v2(source.clone().into(), 0, &caps).unwrap();
    let mut full = Trace::default();
    let observed = admit_sfnt_cff1_v2_owned_with_admission_budget(source, 0, &caps, &mut full)
        .unwrap()
        .unwrap();
    assert_eq!(native.fingerprint(), observed.fingerprint());
    let mut cuts = 0;
    for stop in 0..full.requests.len() {
        let mut short = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        let source = sfnt_cases::replace(TINY, *b"FFTM", Some(metadata.clone()));
        let failure = admit_sfnt_cff1_v2_owned_with_admission_budget(source, 0, &caps, &mut short)
            .unwrap_err();
        assert_eq!(short.requests, full.requests[..=stop]);
        if let CffAdmissionBudgetFailureV2::Sfnt(f) = failure {
            if f.point.stage == CffSfntStageV2::FontForgeMetadata {
                assert_eq!(f.point.table_tag, Some(*b"FFTM"));
                assert_eq!(f.point.table_offset, Some(0));
                assert!(f.point.file_offset.is_some());
            }
        }
        cuts += 1;
    }
    for length in [1, 4, 27, 29] {
        let source = sfnt_cases::replace(
            TINY,
            *b"FFTM",
            Some(metadata.iter().copied().cycle().take(length).collect()),
        );
        assert_eq!(
            admit_sfnt_cff1_v2(source.into(), 0, &caps)
                .unwrap_err()
                .kind,
            Cff1FailureKindV2::Sfnt(Cff1Error::InvalidOptionalTable)
        );
    }
    for version in [0, 2, 255] {
        metadata[3] = version;
        let source = sfnt_cases::replace(TINY, *b"FFTM", Some(metadata.clone()));
        let failure = admit_sfnt_cff1_v2(source.into(), 0, &caps).unwrap_err();
        assert_eq!(failure.context.table_tag, Some(*b"FFTM"));
        assert_eq!(failure.context.phase, FontFailurePhase::TableDecode);
        assert_eq!(failure.context.table_offset, Some(0));
    }
    eprintln!("FFTM legacy negative private positive all callback cuts={cuts}");
}

fn limits() -> M4EffectiveResourceLimits {
    M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap()
}
#[test]
#[ignore = "requires unchanged original fonts through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn cff_v2_name_keyed_original_latin_modern_admission_all_glyphs_and_subset() {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let source = std::fs::read(list.lines().nth(2).unwrap()).unwrap();
    let hex = |bytes: &[u8]| bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
    assert_eq!(
        hex(&sha256(&source)),
        "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f"
    );
    let caps = limits();
    let old = admit_sfnt_cff1_detailed(&source, 0, &caps).unwrap_err();
    assert_eq!(old.kind, Cff1Error::UnsupportedTable);
    assert_eq!(old.context.table_tag, Some(*b"FFTM"));
    let native = admit_sfnt_cff1_v2(source.clone().into(), 0, &caps).unwrap();
    let mut trace = Trace::default();
    let observed =
        admit_sfnt_cff1_v2_owned_with_admission_budget(source.clone(), 0, &caps, &mut trace)
            .unwrap()
            .unwrap();
    assert_eq!(native.fingerprint(), observed.fingerprint());
    assert_eq!(native.source_sha256(), sha256(&source));
    assert_eq!(native.glyph_count(), 4802);
    assert_eq!(native.program().font_dict_count(), 1);
    let sids = (0..4802)
        .flat_map(|g| native.program().sid_for_gid(g).unwrap().to_be_bytes())
        .collect::<Vec<_>>();
    for g in 0..4802 {
        assert_eq!(native.program().cid_for_gid(g), None);
        assert_eq!(native.program().fd_for_gid(g), Some(0));
    }
    for stop in [
        0,
        1,
        trace.requests.len() / 4,
        trace.requests.len() / 2,
        trace.requests.len() * 3 / 4,
        trace.requests.len() - 1,
    ] {
        let mut short = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        assert!(admit_sfnt_cff1_v2_owned_with_admission_budget(
            source.clone(),
            0,
            &caps,
            &mut short
        )
        .is_err());
        assert_eq!(short.requests, trace.requests[..=stop]);
    }
    let mut evaluator = CffProgramEvaluationSessionV2::new(&caps);
    for gid in 0..4802 {
        let advance = native.horizontal_metric(gid).unwrap().0;
        let glyph = evaluator
            .evaluate(native.program(), gid, advance)
            .unwrap_or_else(|e| panic!("original GID {gid}: {e:?}"));
        assert_eq!(glyph.fd(), 0);
    }
    let selected: BTreeSet<_> = "A∑∫√𝑥→("
        .chars()
        .map(|ch| OriginalGlyphId::new(native.cmap().glyph_for_sequence(ch, None).unwrap()))
        .collect();
    let closure = Cff1SubsetSessionV2::close_instance_selection(
        &native,
        FontFaceId::new(0),
        FontInstanceId::new(0),
        &selected,
        100,
    )
    .unwrap();
    let mut session = Cff1SubsetSessionV2::from_admission(&native);
    let subset = session.subset(&native, closure).unwrap();
    // Resource admission deliberately forbids subset-prefixed source names.
    // Independently parse the emitted SFNT wrapper and inspect its CID table.
    use read_fonts::{types::Tag, FontRef, TableProvider};
    let font = FontRef::new(subset.bytes()).unwrap();
    let glyphs = font.maxp().unwrap().num_glyphs();
    assert_eq!(glyphs as usize, selected.len() + 1);
    let table = font.data_for_tag(Tag::new(b"CFF ")).unwrap();
    let cff = font.cff().unwrap();
    let top = cff.top_dicts();
    let mut charset_at = None;
    let mut cid = false;
    for entry in read_fonts::tables::postscript::dict::entries(top.get(0).unwrap(), None) {
        match entry.unwrap() {
            read_fonts::tables::postscript::dict::Entry::Ros { .. } => cid = true,
            read_fonts::tables::postscript::dict::Entry::Charset(at) => charset_at = Some(at),
            _ => {}
        }
    }
    assert!(cid);
    let (ids, _) = super::super::charset(table.as_bytes(), charset_at.unwrap(), glyphs).unwrap();
    assert_eq!(ids, (0..glyphs).collect::<Vec<_>>());
    eprintln!("original Latin Modern bytes={} glyphs=4802 SID-sha256={} requests={} cuts=6 ops={} segments={} subset-bytes={} subset-sha256={}",source.len(),hex(&sha256(&sids)),trace.requests.len(),evaluator.operations_used(),evaluator.outline_segments_used(),subset.bytes().len(),hex(&subset.sha256()));
}

#[test]
fn cff_v2_name_keyed_combined_embedding_bits_have_a_narrow_versioned_policy() {
    for value in 0..=u16::MAX {
        let expected = match value {
            0 => Some(Cff1EmbeddingPermission::Installable),
            4 => Some(Cff1EmbeddingPermission::PreviewAndPrint),
            8 | 12 => Some(Cff1EmbeddingPermission::Editable),
            _ => None,
        };
        assert_eq!(embedding_permission_v2(value).ok(), expected);
        assert_eq!(
            embedding_permission(value).ok(),
            if value == 12 { None } else { expected }
        );
    }
    let caps = limits();
    let records = preflight_sfnt_with_vertical(
        TINY,
        *caps.extension().get(),
        &mut FontFailureContext::new(0),
        false,
    )
    .unwrap();
    let row = records.iter().find(|r| r.tag == *b"OS/2").unwrap();
    let mut os2 = TINY[row.offset..row.offset + row.length].to_vec();
    os2[8..10].copy_from_slice(&12u16.to_be_bytes());
    let source = sfnt_cases::replace(TINY, *b"OS/2", Some(os2.clone()));
    assert_eq!(
        admit_sfnt_cff1_detailed(&source, 0, &caps)
            .unwrap_err()
            .kind,
        Cff1Error::RestrictedEmbedding
    );
    let native = admit_sfnt_cff1_v2(source.into(), 0, &caps).unwrap();
    assert_eq!(
        native.embedding_permission(),
        Cff1EmbeddingPermission::Editable
    );
    // A later source-program failure still reports the private permission policy.
    let source = sfnt_cases::replace(TINY, *b"OS/2", Some(os2));
    let source = sfnt_cases::replace(&source, *b"CFF ", Some(vec![0; 4]));
    let failure = admit_sfnt_cff1_v2(source.into(), 0, &caps).unwrap_err();
    assert_eq!(failure.context.embedding, FontEmbeddingStatus::Allowed(12));
}

#[test]
fn cff_v2_name_keyed_mac_roman_metadata_cannot_supply_unicode_coverage() {
    let caps = limits();
    let records = preflight_sfnt_with_vertical(
        TINY,
        *caps.extension().get(),
        &mut FontFailureContext::new(0),
        false,
    )
    .unwrap();
    let row = records.iter().find(|r| r.tag == *b"cmap").unwrap();
    let old = &TINY[row.offset..row.offset + row.length];
    let count = u16::from_be_bytes(old[2..4].try_into().unwrap()) as usize;
    let mut cmap = vec![0, 0];
    cmap.extend(((count + 1) as u16).to_be_bytes());
    for record in old[4..4 + count * 8].chunks_exact(8) {
        cmap.extend(&record[..4]);
        let offset = u32::from_be_bytes(record[4..].try_into().unwrap()) + 8;
        cmap.extend(offset.to_be_bytes());
    }
    cmap.extend([0, 1, 0, 0]);
    let offset = (old.len() + 8) as u32;
    cmap.extend(offset.to_be_bytes());
    cmap.extend(&old[4 + count * 8..]);
    let start = cmap.len();
    cmap.extend([0, 6, 0, 14, 0, 0, 0, 65, 0, 2, 0, 1, 0, 2]);
    let native = validate_cff_cmap_v2(&cmap, 4).unwrap();
    let base = validate_cff_cmap_v2(old, 4).unwrap();
    assert_eq!(native.base_map(), base.base_map());
    let mut full = Trace::default();
    let observed = validate_cff_cmap_v2_with_base_cmap_budget(&cmap, 4, &mut full)
        .unwrap()
        .unwrap();
    assert_eq!(observed.base_map(), base.base_map());
    for stop in 0..full.requests.len() {
        let mut short = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        let failure = validate_cff_cmap_v2_with_base_cmap_budget(&cmap, 4, &mut short).unwrap_err();
        assert_eq!(failure.error, stop);
        assert_eq!(short.requests, full.requests[..=stop]);
    }
    eprintln!(
        "Mac Roman6 with required Unicode all callback cuts={}",
        full.requests.len()
    );
    assert!(matches!(
        parse_cmap(&cmap, 4, &mut FontFailureContext::new(0)),
        Err(Cff1Error::InvalidCmap)
    ));
    for (at, value) in [
        (start + 3, 12),
        (start + 5, 1),
        (start + 7, 255),
        (start + 9, 3),
        (start + 11, 4),
    ] {
        let mut bad = cmap.clone();
        bad[at] = value;
        assert!(validate_cff_cmap_v2(&bad, 4).is_err());
    }
    let mut only = vec![0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 12];
    only.extend(&cmap[start..]);
    assert!(validate_cff_cmap_v2(&only, 4).is_err());
    for end in start..cmap.len() {
        assert!(validate_cff_cmap_v2(&cmap[..end], 4).is_err());
    }
}
