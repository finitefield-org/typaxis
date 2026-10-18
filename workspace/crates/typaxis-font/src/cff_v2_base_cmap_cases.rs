pub fn format12(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut b = vec![0, 12, 0, 0];
    for n in [16 + 12 * groups.len() as u32, 0, groups.len() as u32] {
        b.extend(n.to_be_bytes());
    }
    for &(start, end, gid) in groups {
        for n in [start, end, gid] {
            b.extend(n.to_be_bytes());
        }
    }
    b
}
pub fn format4(segments: &[(u16, u16, i16, u16)], glyphs: &[u16]) -> Vec<u8> {
    let n = segments.len() as u16;
    let power = 1 << (15 - n.leading_zeros());
    let mut b = Vec::new();
    for v in [
        4u16,
        16 + n * 8 + glyphs.len() as u16 * 2,
        0,
        n * 2,
        power * 2,
        power.trailing_zeros() as u16,
        n * 2 - power * 2,
    ] {
        b.extend(v.to_be_bytes());
    }
    for s in segments {
        b.extend(s.1.to_be_bytes());
    }
    b.extend([0, 0]);
    for s in segments {
        b.extend(s.0.to_be_bytes());
    }
    for s in segments {
        b.extend(s.2.to_be_bytes());
    }
    for s in segments {
        b.extend(s.3.to_be_bytes());
    }
    for &v in glyphs {
        b.extend(v.to_be_bytes());
    }
    b
}
pub fn cmap(records: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
    let mut b = vec![0, 0];
    b.extend((records.len() as u16).to_be_bytes());
    let mut offset = 4 + records.len() * 8;
    for (platform, encoding, bytes) in records {
        b.extend(platform.to_be_bytes());
        b.extend(encoding.to_be_bytes());
        b.extend((offset as u32).to_be_bytes());
        offset += bytes.len();
    }
    for (_, _, bytes) in records {
        b.extend(bytes);
    }
    b
}
pub fn cases() -> Vec<(String, Vec<u8>, u16)> {
    let mut cases = Vec::new();
    for count in [0usize, 1, 2, 3, 4, 7, 16, 31] {
        let a: Vec<_> = (0..count)
            .map(|i| (32 + i as u32 * 2, 32 + i as u32 * 2, 1 + i as u32 % 7))
            .collect();
        let b: Vec<_> = (0..count)
            .map(|i| (33 + i as u32 * 2, 33 + i as u32 * 2, 1 + i as u32 % 7))
            .collect();
        let left = format12(&a);
        let right = format12(&b);
        for (mode, records) in [
            vec![(0, 4, left.clone())],
            vec![(0, 4, left.clone()), (3, 10, right.clone())],
            vec![(3, 10, right.clone()), (0, 4, left.clone())],
            vec![(0, 4, left.clone()), (3, 10, left.clone())],
            vec![
                (3, 10, right.clone()),
                (0, 4, left.clone()),
                (0, 4, left.clone()),
            ],
        ]
        .into_iter()
        .enumerate()
        {
            cases.push((format!("ordered-{count}-{mode}"), cmap(&records), 8));
        }
    }
    let direct = format4(&[(65, 67, -64, 0), (0xffff, 0xffff, 1, 0)], &[]);
    let indexed = format4(&[(65, 67, 1, 4), (0xffff, 0xffff, 1, 0)], &[1, 0, 2]);
    let matching = format12(&[(65, 67, 1), (0x1f600, 0x1f602, 4)]);
    for (label, records) in [
        ("direct4", vec![(3, 1, direct.clone())]),
        ("indexed4", vec![(0, 3, indexed)]),
        (
            "mixed",
            vec![(3, 1, direct.clone()), (3, 10, matching.clone())],
        ),
        (
            "reverse-mixed",
            vec![(3, 10, matching), (3, 1, direct.clone())],
        ),
        (
            "conflict",
            vec![(3, 1, direct), (0, 4, format12(&[(66, 66, 7)]))],
        ),
        (
            "bad-before-conflict",
            vec![
                (0, 4, format12(&[(65, 66, 1)])),
                (3, 10, format12(&[(65, 65, 7)])),
                (3, 10, vec![0, 2]),
            ],
        ),
        (
            "duplicate-scalar",
            vec![(0, 4, format12(&[(65, 65, 1), (65, 65, 1)]))],
        ),
        ("surrogate", vec![(3, 10, format12(&[(0xd7ff, 0xd800, 1)]))]),
        (
            "unicode-end",
            vec![(0, 4, format12(&[(0x10fffe, 0x10ffff, 1)]))],
        ),
        ("zero-gid", vec![(3, 10, format12(&[(65, 67, 0)]))]),
        (
            "gid-overflow",
            vec![(3, 10, format12(&[(65, 67, u32::MAX)]))],
        ),
        ("no-records", vec![]),
    ] {
        cases.push((label.into(), cmap(&records), 8));
    }
    let mut alias = cmap(&[(0, 4, format12(&[(65, 67, 1)])), (3, 10, Vec::new())]);
    let offset: [u8; 4] = alias[8..12].try_into().unwrap();
    alias[16..20].copy_from_slice(&offset);
    cases.push(("offset-alias".into(), alias, 8));
    let mut variation = vec![0, 14];
    variation.extend(30u32.to_be_bytes());
    variation.extend(1u32.to_be_bytes());
    variation.extend([0xe, 1, 0]);
    variation.extend(0u32.to_be_bytes());
    variation.extend(21u32.to_be_bytes());
    variation.extend(1u32.to_be_bytes());
    variation.extend([0, 0x4e, 0]);
    variation.extend(4u16.to_be_bytes());
    for base in [format12(&[]), format12(&[(0x4e00, 0x4e00, 1)])] {
        cases.push((
            format!("supplement-{}", base.len()),
            cmap(&[(0, 4, base), (0, 5, variation.clone())]),
            8,
        ));
    }
    cases
}
