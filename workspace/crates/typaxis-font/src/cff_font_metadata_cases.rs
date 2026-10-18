pub type NameRecord = (u16, u16, u16, u16, Vec<u8>);
pub fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_be_bytes).collect()
}
pub fn name(records: &[NameRecord]) -> Vec<u8> {
    let mut b = vec![0u8; 6 + records.len() * 12];
    b[2..4].copy_from_slice(&(records.len() as u16).to_be_bytes());
    let start = b.len();
    b[4..6].copy_from_slice(&(start as u16).to_be_bytes());
    for (i, (p, e, l, id, data)) in records.iter().enumerate() {
        let at = 6 + i * 12;
        for (j, n) in [*p, *e, *l, *id, data.len() as u16, (b.len() - start) as u16]
            .iter()
            .enumerate()
        {
            b[at + j * 2..at + j * 2 + 2].copy_from_slice(&n.to_be_bytes());
        }
        b.extend(data);
    }
    b
}
pub fn standard() -> Vec<NameRecord> {
    vec![
        (3, 10, 0x409, 1, utf16("Family")),
        (3, 10, 0x409, 2, utf16("Regular")),
        (3, 10, 0x409, 6, utf16("Family-Regular")),
    ]
}
pub fn names() -> Vec<(String, Vec<u8>)> {
    let mut cases = vec![("standard".into(), name(&standard()))];
    for value in [
        "",
        "日本語",
        "éΩ",
        "A\nB",
        "A\u{7f}",
        "😀",
        "ABCDEF+Subset",
        "ABCDE+Okay",
        "Has Space",
        "Name/Bad",
    ] {
        for id in [1, 2, 6] {
            let mut r = standard();
            r.iter_mut().find(|r| r.3 == id).unwrap().4 = utf16(value);
            cases.push((format!("id-{id}-value-{value:?}"), name(&r)));
        }
    }
    for payload in [
        vec![0],
        vec![0xd8, 0],
        vec![0xdc, 0],
        vec![0xd8, 0, 0xdc, 0],
        vec![0, 65],
    ] {
        for (platform, encoding, language, id) in [
            (3, 10, 0x409, 1),
            (0, 0, 0, 1),
            (1, 0, 0, 1),
            (3, 0, 0, 1),
            (3, 10, 0x409, 99),
        ] {
            let mut r = standard();
            r.push((platform, encoding, language, id, payload.clone()));
            cases.push((
                format!("extra-{platform}-{encoding}-{language}-{id}-{payload:?}"),
                name(&r),
            ));
        }
    }
    for rank in [
        (3, 10, 0x409),
        (3, 1, 0x409),
        (0, 0, 0),
        (0, 1, 0),
        (3, 10, 0x411),
        (3, 1, 0x411),
    ] {
        for front in [true, false] {
            let mut r = standard();
            let extra = (rank.0, rank.1, rank.2, 1, utf16("Other"));
            if front {
                r.insert(0, extra);
            } else {
                r.push(extra);
            }
            cases.push((format!("rank-{rank:?}-front-{front}"), name(&r)));
        }
    }
    let good = name(&standard());
    for end in 0..good.len() {
        cases.push((format!("prefix-{end}"), good[..end].to_vec()));
    }
    for i in 0..good.len() {
        for v in [0, 1, 255] {
            if good[i] != v {
                let mut b = good.clone();
                b[i] = v;
                cases.push((format!("byte-{i}-{v}"), b));
            }
        }
    }
    cases
}
pub fn metrics() -> Vec<(u16, u16, Vec<u8>)> {
    let mut out = Vec::new();
    for glyphs in [0u16, 1, 2, 4, 17] {
        for n in [0, 1, 2, 4, 18] {
            let expected = usize::from(n) * 4 + usize::from(glyphs.saturating_sub(n)) * 2;
            for len in [0, 1, expected.saturating_sub(1), expected, expected + 1] {
                out.push((
                    glyphs,
                    n,
                    (0..len).map(|i| ((i * 73 + 11) % 256) as u8).collect(),
                ));
            }
        }
    }
    out
}
pub fn fixture(names: Vec<u8>, metrics: Vec<u8>, glyphs: u16, n: u16) -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[..4].copy_from_slice(&0x10000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5f0f3cf5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[40..42].copy_from_slice(&1000i16.to_be_bytes());
    head[42..44].copy_from_slice(&1000i16.to_be_bytes());
    let mut maxp = 0x5000u32.to_be_bytes().to_vec();
    maxp.extend(glyphs.to_be_bytes());
    let mut hhea = vec![0u8; 36];
    hhea[..4].copy_from_slice(&0x10000u32.to_be_bytes());
    hhea[34..].copy_from_slice(&n.to_be_bytes());
    let mut cmap = vec![0u8; 12];
    cmap[2..4].copy_from_slice(&1u16.to_be_bytes());
    cmap[4..6].copy_from_slice(&3u16.to_be_bytes());
    cmap[6..8].copy_from_slice(&10u16.to_be_bytes());
    cmap[8..12].copy_from_slice(&12u32.to_be_bytes());
    cmap.extend([0, 12, 0, 0]);
    for v in [28u32, 0, 1, 65, 65, 1] {
        cmap.extend(v.to_be_bytes());
    }
    let mut post = vec![0u8; 32];
    post[..4].copy_from_slice(&0x30000u32.to_be_bytes());
    let tables = vec![
        (*b"CFF ", vec![1, 0, 4, 4, 0, 0]),
        (*b"OS/2", vec![0u8; 78]),
        (*b"cmap", cmap),
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"hmtx", metrics),
        (*b"maxp", maxp),
        (*b"name", names),
        (*b"post", post),
    ];
    let mut bytes = vec![0u8; 12 + tables.len() * 16];
    bytes[..4].copy_from_slice(b"OTTO");
    bytes[4..6].copy_from_slice(&9u16.to_be_bytes());
    bytes[6..8].copy_from_slice(&128u16.to_be_bytes());
    bytes[8..10].copy_from_slice(&3u16.to_be_bytes());
    bytes[10..12].copy_from_slice(&16u16.to_be_bytes());
    let mut head_start = 0;
    for (i, (tag, data)) in tables.into_iter().enumerate() {
        let offset = bytes.len();
        let at = 12 + i * 16;
        bytes[at..at + 4].copy_from_slice(&tag);
        bytes[at + 4..at + 8].copy_from_slice(&checksum(&data).to_be_bytes());
        bytes[at + 8..at + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        bytes[at + 12..at + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        if tag == *b"head" {
            head_start = offset;
        }
        bytes.extend(data);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
    }
    let sum = checksum(&bytes);
    bytes[head_start + 8..head_start + 12]
        .copy_from_slice(&0xb1b0_afbau32.wrapping_sub(sum).to_be_bytes());
    bytes
}
fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(w))
    })
}
pub fn cases() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (label, b) in names() {
        out.push((format!("name-{label}"), fixture(b, vec![0; 10], 4, 1)));
    }
    for (i, (g, n, b)) in metrics().into_iter().enumerate() {
        out.push((format!("metrics-{i}"), fixture(name(&standard()), b, g, n)));
    }
    out
}
