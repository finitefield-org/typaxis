use super::*;
fn header(n: u16) -> Vec<u8> {
    let mut h = vec![0; 36];
    h[..4].copy_from_slice(&0x00011000u32.to_be_bytes());
    h[34..].copy_from_slice(&n.to_be_bytes());
    h
}
#[test]
fn cff_v2_vertical_sparse_origins_and_trailing_metrics_are_exact() {
    let v = [0, 1, 0, 0, 3, 112, 0, 1, 0, 1, 3, 212];
    let h = header(2);
    let m = [3, 232, 0, 10, 1, 244, 255, 246, 0, 30];
    let p = validate_cff_vertical_metrics_v2(3, Some(&v), Some(&h), Some(&m)).unwrap();
    assert_eq!(p.long_metric_count(), 2);
    assert_eq!(p.origin_record_count(), 1);
    assert_eq!(p.vertical_origin(0), Some(880));
    assert_eq!(p.vertical_origin(1), Some(980));
    assert_eq!(p.vertical_origin(2), Some(880));
    assert_eq!(p.vertical_origin(3), None);
    assert_eq!(p.glyph_metric(0), Some((1000, 10)));
    assert_eq!(p.glyph_metric(1), Some((500, -10)));
    assert_eq!(p.glyph_metric(2), Some((500, 30)));
    assert_eq!(p.glyph_metric(3), None);
}
#[test]
fn cff_v2_vertical_tables_are_optional_but_metric_pair_is_required() {
    let p = validate_cff_vertical_metrics_v2(1, None, None, None).unwrap();
    assert_eq!(p.glyph_metric(0), None);
    assert_eq!(p.vertical_origin(0), None);
    for (h, m, missing) in [
        (Some(&[][..]), None, *b"vmtx"),
        (None, Some(&[][..]), *b"vhea"),
    ] {
        let e = validate_cff_vertical_metrics_v2(1, None, h, m).unwrap_err();
        assert_eq!((e.table, e.kind), (missing, K::MissingTablePair));
    }
}
#[test]
fn cff_v2_vertical_rejects_every_truncation_and_trailing_data() {
    let v = [0, 1, 0, 0, 3, 112, 0, 1, 0, 0, 3, 212];
    let h = header(1);
    let m = [3, 232, 0, 10];
    for i in 0..v.len() {
        assert!(validate_cff_vertical_metrics_v2(1, Some(&v[..i]), None, None).is_err());
    }
    for i in 0..h.len() {
        assert!(validate_cff_vertical_metrics_v2(1, None, Some(&h[..i]), Some(&m)).is_err());
    }
    for i in 0..m.len() {
        assert!(validate_cff_vertical_metrics_v2(1, None, Some(&h), Some(&m[..i])).is_err());
    }
    let mut trailing = v.to_vec();
    trailing.push(0);
    assert!(validate_cff_vertical_metrics_v2(1, Some(&trailing), None, None).is_err());
    let mut trailing = m.to_vec();
    trailing.push(0);
    assert!(validate_cff_vertical_metrics_v2(1, None, Some(&h), Some(&trailing)).is_err());
}
#[test]
fn cff_v2_vertical_rejects_unordered_and_out_of_range_origins() {
    let mut v = vec![0, 1, 0, 0, 3, 112, 0, 2, 0, 1, 3, 212, 0, 2, 3, 112];
    v[13] = 1;
    assert_eq!(
        validate_cff_vertical_metrics_v2(3, Some(&v), None, None)
            .unwrap_err()
            .kind,
        K::InvalidGlyphOrder
    );
    v[13] = 3;
    assert_eq!(
        validate_cff_vertical_metrics_v2(3, Some(&v), None, None)
            .unwrap_err()
            .kind,
        K::GlyphOutOfRange
    );
    v[1] = 2;
    assert_eq!(
        validate_cff_vertical_metrics_v2(3, Some(&v), None, None)
            .unwrap_err()
            .kind,
        K::UnsupportedVersion
    );
}
#[test]
fn cff_v2_vertical_checks_version_reserved_fields_and_metric_count() {
    let h = header(1);
    let m = [3, 232, 0, 10];
    for at in (24..=32).step_by(2) {
        let mut h = h.clone();
        h[at + 1] = 1;
        let e = validate_cff_vertical_metrics_v2(1, None, Some(&h), Some(&m)).unwrap_err();
        assert_eq!((e.kind, e.table_offset), (K::InvalidReservedField, at));
    }
    for n in [0, 2] {
        assert_eq!(
            validate_cff_vertical_metrics_v2(1, None, Some(&header(n)), Some(&m))
                .unwrap_err()
                .kind,
            K::InvalidMetricCount
        );
    }
    let mut h = h.clone();
    h[..4].copy_from_slice(&0x00010000u32.to_be_bytes());
    assert!(validate_cff_vertical_metrics_v2(1, None, Some(&h), Some(&m)).is_ok());
    h[9] = 1;
    assert_eq!(
        validate_cff_vertical_metrics_v2(1, None, Some(&h), Some(&m))
            .unwrap_err()
            .table_offset,
        8
    );
    h[..4].copy_from_slice(&0x00010001u32.to_be_bytes());
    assert_eq!(
        validate_cff_vertical_metrics_v2(1, None, Some(&h), Some(&m))
            .unwrap_err()
            .kind,
        K::UnsupportedVersion
    );
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_vertical_original_harano_all_metrics() {
    use read_fonts::{types::Tag, FontRef, TableProvider};
    use typaxis_core::sha256;
    let source = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    assert_eq!(
        hex(&sha256(&source)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let font = FontRef::new(&source).unwrap();
    let v = font.data_for_tag(Tag::new(b"VORG")).unwrap();
    let h = font.data_for_tag(Tag::new(b"vhea")).unwrap();
    let m = font.data_for_tag(Tag::new(b"vmtx")).unwrap();
    let glyphs = font.maxp().unwrap().num_glyphs();
    let p = validate_cff_vertical_metrics_v2(
        glyphs,
        Some(v.as_bytes()),
        Some(h.as_bytes()),
        Some(m.as_bytes()),
    )
    .unwrap();
    assert_eq!(p.long_metric_count(), 21012);
    assert_eq!(p.origin_record_count(), 152);
    assert_eq!(p.header_version(), Some(0x00011000));
    let mut bytes = Vec::new();
    for gid in 0..glyphs {
        let (advance, top) = p.glyph_metric(gid).unwrap();
        bytes.extend(p.vertical_origin(gid).unwrap().to_be_bytes());
        bytes.extend(advance.to_be_bytes());
        bytes.extend(top.to_be_bytes());
    }
    assert_eq!(
        hex(&sha256(&bytes)),
        "a48a869c98cf13ff94cd763ccabf2dbdf849bedc531ea46126a36a075db6c676"
    );
}
