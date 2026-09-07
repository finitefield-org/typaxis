use super::*;
fn put24(v: u32, b: &mut Vec<u8>) {
    b.extend(&v.to_be_bytes()[1..]);
}
fn table() -> Vec<u8> {
    let mut b = vec![0, 14, 0, 0, 0, 0, 0, 0, 0, 1];
    put24(0xe0100, &mut b);
    b.extend(21u32.to_be_bytes());
    b.extend(29u32.to_be_bytes());
    b.extend(1u32.to_be_bytes());
    put24(0x4e00, &mut b);
    b.push(2);
    b.extend(1u32.to_be_bytes());
    put24(0x4e03, &mut b);
    b.extend(4u16.to_be_bytes());
    let len = b.len() as u32;
    b[2..6].copy_from_slice(&len.to_be_bytes());
    b
}
fn cmap(table: &[u8]) -> Vec<u8> {
    let mut b = vec![0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0, 12];
    b.extend(table);
    b
}
#[test]
fn cff_v2_variations_distinguish_default_nondefault_and_missing_pairs() {
    let b = cmap(&table());
    let p = validate_cff_variation_sequences_v2(&b, 5).unwrap().unwrap();
    assert_eq!(p.selector_count(), 1);
    assert_eq!(p.declared_value_count(), 4);
    for c in ['\u{4e00}', '\u{4e01}', '\u{4e02}'] {
        assert_eq!(p.coverage(c, '\u{e0100}'), VariationCoverage::Default);
    }
    assert_eq!(
        p.coverage('\u{4e03}', '\u{e0100}'),
        VariationCoverage::NonDefault(4)
    );
    assert_eq!(
        p.coverage('\u{4e04}', '\u{e0100}'),
        VariationCoverage::Missing
    );
    assert_eq!(
        p.coverage('\u{4e00}', '\u{e0101}'),
        VariationCoverage::Missing
    );
    assert!(validate_cff_variation_sequences_v2(&[0, 0, 0, 0], 5)
        .unwrap()
        .is_none());
}
#[test]
fn cff_v2_variations_reject_all_truncations_and_wrong_encoding() {
    let b = cmap(&table());
    for end in 0..b.len() {
        assert!(validate_cff_variation_sequences_v2(&b[..end], 5).is_err());
    }
    let mut bad = b.clone();
    bad[7] = 4;
    assert_eq!(
        validate_cff_variation_sequences_v2(&bad, 5)
            .unwrap_err()
            .kind,
        K::InvalidCmapEncoding
    );
    let mut bad = b.clone();
    bad[13] = 12;
    assert_eq!(
        validate_cff_variation_sequences_v2(&bad, 5)
            .unwrap_err()
            .kind,
        K::InvalidCmapEncoding
    );
}
#[test]
fn cff_v2_variations_reject_intersections_invalid_gids_and_surrogate_ranges() {
    let mut b = table();
    b[35] = 1;
    assert_eq!(
        parse_format14(&b, 5).unwrap_err().kind,
        K::InvalidVariationMapping
    );
    let mut b = table();
    b[37] = 5;
    assert_eq!(parse_format14(&b, 5).unwrap_err().kind, K::GlyphOutOfRange);
    let mut b = table();
    b[25..28].copy_from_slice(&[0, 0xd7, 0xff]);
    assert_eq!(
        parse_format14(&b, 5).unwrap_err().kind,
        K::InvalidUnicodeRange
    );
    let mut b = table();
    b[10..13].copy_from_slice(&[0, 0, 65]);
    assert_eq!(
        parse_format14(&b, 5).unwrap_err().kind,
        K::InvalidVariationSelector
    );
    let mut b = table();
    b[13..17].copy_from_slice(&10u32.to_be_bytes());
    assert_eq!(parse_format14(&b, 5).unwrap_err().kind, K::OverlappingData);
}
#[test]
fn cff_v2_variations_zero_offsets_are_absent_and_selector_limit_precedes_payload() {
    let mut b = table();
    b[13..21].fill(0);
    let p = parse_format14(&b, 5).unwrap();
    assert_eq!(p.declared_value_count(), 0);
    assert_eq!(
        p.coverage('\u{4e00}', '\u{e0100}'),
        VariationCoverage::Missing
    );
    b[6..10].copy_from_slice(&257u32.to_be_bytes());
    let e = parse_format14(&b, 5).unwrap_err();
    assert_eq!(
        (e.kind, e.limit, e.observed),
        (K::UnsupportedComplexity, Some(256), Some(257))
    );
}
fn many_defaults(values: usize) -> Vec<u8> {
    let mut b = vec![0, 14, 0, 0, 0, 0, 0, 0, 0, 1];
    put24(0xe0100, &mut b);
    b.extend(21u32.to_be_bytes());
    b.extend(0u32.to_be_bytes());
    b.extend(0u32.to_be_bytes());
    let mut remaining = values;
    let mut point = 0u32;
    let mut records = 0u32;
    while remaining > 0 {
        if point == 0xd800 {
            point = 0xe000;
        }
        let count = remaining.min(256);
        put24(point, &mut b);
        b.push((count - 1) as u8);
        point += count as u32;
        remaining -= count;
        records += 1;
    }
    b[21..25].copy_from_slice(&records.to_be_bytes());
    let len = b.len() as u32;
    b[2..6].copy_from_slice(&len.to_be_bytes());
    b
}
#[test]
fn cff_v2_variations_shared_payload_is_charged_per_selector_and_order_is_strict() {
    // Two selectors share exactly the same default and non-default data.
    let mut b = table();
    b.splice(21..21, [0u8; 11]);
    b[6..10].copy_from_slice(&2u32.to_be_bytes());
    b[13..17].copy_from_slice(&32u32.to_be_bytes());
    b[17..21].copy_from_slice(&40u32.to_be_bytes());
    let record = b[10..21].to_vec();
    b[21..32].copy_from_slice(&record);
    b[23] = 1;
    let len = b.len() as u32;
    b[2..6].copy_from_slice(&len.to_be_bytes());
    let p = parse_format14(&b, 5).unwrap();
    assert_eq!(p.declared_value_count(), 8);
    assert_eq!(
        p.coverage('\u{4e03}', '\u{e0101}'),
        VariationCoverage::NonDefault(4)
    );
    b[23] = 0;
    assert_eq!(
        parse_format14(&b, 5).unwrap_err().kind,
        K::InvalidVariationSelector
    );
    b[23] = 1;
    // A default range may not alias a non-default mapping, even when the
    // shared bytes happen to be structurally valid as both record kinds.
    b[24..28].copy_from_slice(&40u32.to_be_bytes());
    b[28..32].fill(0);
    assert_eq!(parse_format14(&b, 5).unwrap_err().kind, K::OverlappingData);
    let e = validate_cff_variation_sequences_v2(&cmap(&table()), 0).unwrap_err();
    assert_eq!(
        (e.table, e.table_offset, e.kind),
        (*b"maxp", 4, K::InvalidMetricCount)
    );
}
#[test]
fn cff_v2_variations_expanded_value_limit_is_inclusive_without_expansion() {
    let b = many_defaults(1_000_000);
    assert_eq!(
        parse_format14(&b, 5).unwrap().declared_value_count(),
        1_000_000
    );
    let b = many_defaults(1_000_001);
    let e = parse_format14(&b, 5).unwrap_err();
    assert_eq!(
        (e.kind, e.limit, e.observed),
        (K::UnsupportedComplexity, Some(1_000_000), Some(1_000_001))
    );
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_variations_original_harano_all_sequences() {
    use read_fonts::{types::Tag, FontRef, TableProvider};
    use typaxis_core::sha256;
    let source = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    assert_eq!(
        hex(&sha256(&source)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let font = FontRef::new(&source).unwrap();
    let cmap = font.data_for_tag(Tag::new(b"cmap")).unwrap();
    let glyphs = font.maxp().unwrap().num_glyphs();
    let p = validate_cff_variation_sequences_v2(cmap.as_bytes(), glyphs)
        .unwrap()
        .unwrap();
    assert_eq!(p.selector_count(), 17);
    assert_eq!(p.declared_value_count(), 14780);
    let mut records = std::collections::BTreeMap::new();
    let b = p.table;
    for i in 0..p.records {
        let at = 10 + i * 11;
        let selector = u24_at(b, at);
        for (field, stride) in [(at + 3, 4), (at + 7, 5)] {
            let offset = u32_at(b, field) as usize;
            if offset == 0 {
                continue;
            }
            for j in 0..u32_at(b, offset) as usize {
                let at = offset + 4 + j * stride;
                let base = u24_at(b, at);
                let extra = if stride == 4 { u32::from(b[at + 3]) } else { 0 };
                for scalar in base..=base + extra {
                    records.insert(
                        (selector, scalar),
                        p.coverage(
                            char::from_u32(scalar).unwrap(),
                            char::from_u32(selector).unwrap(),
                        ),
                    );
                }
            }
        }
    }
    let mut bytes = Vec::new();
    for ((selector, base), coverage) in records {
        bytes.extend(selector.to_be_bytes());
        bytes.extend(base.to_be_bytes());
        match coverage {
            VariationCoverage::Default => {
                bytes.push(0);
                bytes.extend(0u16.to_be_bytes());
            }
            VariationCoverage::NonDefault(gid) => {
                bytes.push(1);
                bytes.extend(gid.to_be_bytes());
            }
            VariationCoverage::Missing => panic!("listed pair missing"),
        }
    }
    assert_eq!(
        hex(&sha256(&bytes)),
        "ed20f9a7d92d9331403ef147385f2025670839d3af63f3af8e91265567c789a8"
    );
}
