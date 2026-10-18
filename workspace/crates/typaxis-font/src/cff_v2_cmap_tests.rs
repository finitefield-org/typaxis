use super::*;
fn fixture() -> Vec<u8> {
    let mut b = vec![0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 20, 0, 0, 0, 5, 0, 0, 0, 48];
    b.extend([0, 12, 0, 0]);
    for n in [28u32, 0, 1, 0x4e00, 0x4e00, 1] {
        b.extend(n.to_be_bytes());
    }
    b.extend([0, 14]);
    b.extend(38u32.to_be_bytes());
    b.extend(1u32.to_be_bytes());
    b.extend([0xe, 1, 0]);
    b.extend(21u32.to_be_bytes());
    b.extend(29u32.to_be_bytes());
    b.extend(1u32.to_be_bytes());
    b.extend([0, 0x4e, 0, 2]);
    b.extend(1u32.to_be_bytes());
    b.extend([0, 0x4e, 3]);
    b.extend(4u16.to_be_bytes());
    b
}
#[test]
fn cff_v2_cmap_uses_base_only_for_default_and_never_drops_selectors() {
    let b = fixture();
    let p = validate_cff_cmap_v2(&b, 5).unwrap();
    assert_eq!(p.base_mapping_count(), 1);
    assert_eq!(p.glyph_for_sequence('\u{4e00}', None), Some(1));
    assert_eq!(p.glyph_for_sequence('\u{4e00}', Some('\u{e0100}')), Some(1));
    assert_eq!(p.glyph_for_sequence('\u{4e01}', Some('\u{e0100}')), None);
    assert_eq!(p.glyph_for_sequence('\u{4e03}', None), None);
    assert_eq!(p.glyph_for_sequence('\u{4e03}', Some('\u{e0100}')), Some(4));
    assert_eq!(p.glyph_for_sequence('\u{4e00}', Some('\u{e0101}')), None);
    assert_eq!(p.glyph_for_sequence('\u{4e00}', Some('A')), None);
    assert_eq!(p.glyph_for_sequence('\u{e0100}', None), None);
    let mut b = b;
    let n = b.len();
    b[n - 1] = 0;
    assert_eq!(
        validate_cff_cmap_v2(&b, 5)
            .unwrap()
            .glyph_for_sequence('\u{4e03}', Some('\u{e0100}')),
        None
    );
}
#[test]
fn cff_v2_cmap_supplement_does_not_make_missing_or_invalid_base_valid() {
    let b = fixture();
    let mut context = FontFailureContext::new(0);
    assert_eq!(
        parse_cmap(&b, 5, &mut context).unwrap_err(),
        Cff1Error::InvalidCmap
    );
    let mut bad = b.clone();
    bad[47] = 5;
    assert!(matches!(
        validate_cff_cmap_v2(&bad, 5),
        Err(CffCmapFailureV2::Base { .. })
    ));
    let mut bad = b.clone();
    bad[13] = 3;
    assert!(matches!(
        validate_cff_cmap_v2(&bad, 5),
        Err(CffCmapFailureV2::Variation(_))
    ));
    // Both records point to the supplemental table. There is no base cmap.
    let mut bad = b;
    bad[7] = 5;
    bad[11] = 48;
    assert!(validate_cff_cmap_v2(&bad, 5).is_err());
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_cmap_original_harano_base_and_resolved_sequences() {
    use read_fonts::{types::Tag, FontRef, TableProvider};
    let source = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    assert_eq!(
        hex(&sha256(&source)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let font = FontRef::new(&source).unwrap();
    let data = font.data_for_tag(Tag::new(b"cmap")).unwrap();
    let p = validate_cff_cmap_v2(data.as_bytes(), font.maxp().unwrap().num_glyphs()).unwrap();
    let mut bytes = Vec::new();
    for &(base, gid) in &p.base {
        bytes.extend(base.to_be_bytes());
        bytes.extend(gid.to_be_bytes());
    }
    assert_eq!(p.base_mapping_count(), 15815);
    assert_eq!(
        hex(&sha256(&bytes)),
        "95df78a2d881b8387c208f4f9540bf86a292855a0a6beb4d1b7fa17fad2baecf"
    );
    let cmap = font.cmap().unwrap();
    let mut pairs = std::collections::BTreeSet::new();
    // Enumerate the exact original listed keys through the separately tested
    // raw format-14 representation; resolve every key with this owner.
    let raw = data.as_bytes();
    for record in cmap.encoding_records() {
        if record.platform_id() != read_fonts::tables::cmap::PlatformId::Unicode
            || record.encoding_id() != 5
        {
            continue;
        }
        let start = record.subtable_offset().to_u32() as usize;
        let b = &raw[start..];
        let u32at = |at| u32::from_be_bytes(b[at..at + 4].try_into().unwrap());
        let u24at = |at| u32::from_be_bytes([0, b[at], b[at + 1], b[at + 2]]);
        for i in 0..u32at(6) as usize {
            let at = 10 + i * 11;
            let selector = u24at(at);
            for (field, stride) in [(at + 3, 4), (at + 7, 5)] {
                let off = u32at(field) as usize;
                if off == 0 {
                    continue;
                }
                for j in 0..u32at(off) as usize {
                    let pos = off + 4 + j * stride;
                    let base = u24at(pos);
                    let extra = if stride == 4 {
                        u32::from(b[pos + 3])
                    } else {
                        0
                    };
                    for base in base..=base + extra {
                        pairs.insert((selector, base));
                    }
                }
            }
        }
    }
    let mut bytes = Vec::new();
    for (selector, base) in pairs {
        let gid = p
            .glyph_for_sequence(
                char::from_u32(base).unwrap(),
                Some(char::from_u32(selector).unwrap()),
            )
            .unwrap();
        bytes.extend(selector.to_be_bytes());
        bytes.extend(base.to_be_bytes());
        bytes.extend(gid.to_be_bytes());
    }
    assert_eq!(
        hex(&sha256(&bytes)),
        "825b70e8ec61dc7e6b9b6909910cdbb30a23e25f203b1f1f7b1aa2c429132214"
    );
}

#[test]
fn cff_v2_cmap_empty_base_requires_usable_nondefault_variation() {
    let mut b = fixture();
    b.drain(36..48);
    b[24..28].copy_from_slice(&16u32.to_be_bytes());
    b[32..36].fill(0);
    b[16..20].copy_from_slice(&36u32.to_be_bytes());
    let p = validate_cff_cmap_v2(&b, 5).unwrap();
    assert_eq!(p.base_mapping_count(), 0);
    assert_eq!(p.glyph_for_sequence('\u{4e03}', Some('\u{e0100}')), Some(4));
    assert_eq!(p.glyph_for_sequence('\u{4e00}', Some('\u{e0100}')), None);
    let mut mapping = BTreeMap::new();
    mapping.insert(OriginalGlyphId::new(4), SubsetGlyphId::new(1));
    let bytes = super::super::subset::build_cmap(&p, &mapping).unwrap();
    let p = validate_cff_cmap_v2(&bytes, 2).unwrap();
    assert_eq!(p.base_mapping_count(), 0);
    assert_eq!(p.glyph_for_sequence('\u{4e03}', Some('\u{e0100}')), Some(1));
    *b.last_mut().unwrap() = 0;
    assert!(validate_cff_cmap_v2(&b, 5).is_err());
}

fn budget_fixture(count: usize) -> Vec<u8> {
    let original = fixture();
    let mut bytes = original[..20].to_vec();
    let size = 16 + 12 * count;
    bytes[16..20].copy_from_slice(&((20 + size) as u32).to_be_bytes());
    bytes.extend([0, 12, 0, 0]);
    bytes.extend((size as u32).to_be_bytes());
    bytes.extend(0u32.to_be_bytes());
    bytes.extend((count as u32).to_be_bytes());
    for index in 0..count {
        let scalar = 0x4e00 + 2 * index as u32;
        for value in [scalar, scalar, 1 + (index as u32 % 4)] {
            bytes.extend(value.to_be_bytes());
        }
    }
    bytes.extend(&original[48..]);
    bytes
}
#[test]
fn cff_v2_cmap_paid_search_matches_independent_mappings_and_stops_at_every_visit() {
    let mut cases = 0;
    let mut cutoffs = 0;
    for count in [0usize, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 127] {
        let cmap = validate_cff_cmap_v2(&budget_fixture(count), 5).unwrap();
        let expected_map = (0..count)
            .map(|i| (0x4e00 + 2 * i as u32, (1 + i % 4) as u16))
            .collect::<BTreeMap<_, _>>();
        let scalars = (0x4df0..0x4e00 + 2 * count as u32 + 5).chain([0xfe00, 0xe0100, 0x10ffff]);
        for scalar in scalars {
            let base = char::from_u32(scalar).unwrap();
            for selector in [None, Some('\u{e0100}'), Some('\u{fe00}'), Some('A')] {
                let expected = if is_selector(base) {
                    None
                } else {
                    match selector {
                        None => expected_map.get(&scalar).copied(),
                        Some('\u{e0100}') if scalar == 0x4e03 => Some(4),
                        Some('\u{e0100}') if (0x4e00..=0x4e02).contains(&scalar) => {
                            expected_map.get(&scalar).copied()
                        }
                        _ => None,
                    }
                };
                let mut visits = 0;
                let actual = cmap
                    .glyph_for_sequence_with_work(base, selector, &mut || {
                        visits += 1;
                        Ok::<_, usize>(())
                    })
                    .unwrap();
                assert_eq!(actual, expected);
                assert_eq!(cmap.glyph_for_sequence(base, selector), expected);
                assert!(visits > 0);
                for stop in 0..visits {
                    let mut paid = 0;
                    let refused = cmap.glyph_for_sequence_with_work(base, selector, &mut || {
                        let index = paid;
                        paid += 1;
                        if index == stop {
                            Err(index)
                        } else {
                            Ok(())
                        }
                    });
                    assert_eq!(refused, Err(stop));
                    assert_eq!(paid, stop + 1);
                    cutoffs += 1;
                }
                cases += 1;
            }
        }
    }
    eprintln!("paid cmap independent cases={cases}, refusal cutoffs={cutoffs}");
}
