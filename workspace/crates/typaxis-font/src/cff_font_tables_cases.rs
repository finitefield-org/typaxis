pub fn standard() -> Vec<u8> {
    vec![
        79, 84, 84, 79, 0, 9, 0, 128, 0, 3, 0, 16, 67, 70, 70, 32, 1, 0, 4, 4, 0, 0, 0, 156, 0, 0,
        0, 6, 79, 83, 47, 50, 0, 0, 0, 0, 0, 0, 0, 164, 0, 0, 0, 78, 99, 109, 97, 112, 0, 15, 0,
        183, 0, 0, 0, 244, 0, 0, 0, 40, 104, 101, 97, 100, 98, 248, 68, 197, 0, 0, 1, 28, 0, 0, 0,
        54, 104, 104, 101, 97, 0, 1, 0, 1, 0, 0, 1, 84, 0, 0, 0, 36, 104, 109, 116, 120, 0, 0, 0,
        0, 0, 0, 1, 120, 0, 0, 0, 10, 109, 97, 120, 112, 0, 4, 80, 0, 0, 0, 1, 132, 0, 0, 0, 6,
        110, 97, 109, 101, 5, 124, 17, 237, 0, 0, 1, 140, 0, 0, 0, 96, 112, 111, 115, 116, 0, 3, 0,
        0, 0, 0, 1, 236, 0, 0, 0, 32, 1, 0, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 3, 0, 10, 0, 0, 0, 12, 0, 12, 0, 0, 0, 0, 0, 28, 0,
        0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 65, 0, 0, 0, 65, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 19, 196,
        140, 57, 95, 15, 60, 245, 0, 0, 3, 232, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 3, 232, 3, 232, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 0, 0, 4, 0, 0, 0, 0, 0, 3, 0, 42, 0, 3, 0, 10, 4, 9, 0,
        1, 0, 12, 0, 0, 0, 3, 0, 10, 4, 9, 0, 2, 0, 14, 0, 12, 0, 3, 0, 10, 4, 9, 0, 6, 0, 28, 0,
        26, 0, 70, 0, 97, 0, 109, 0, 105, 0, 108, 0, 121, 0, 82, 0, 101, 0, 103, 0, 117, 0, 108, 0,
        97, 0, 114, 0, 70, 0, 97, 0, 109, 0, 105, 0, 108, 0, 121, 0, 45, 0, 82, 0, 101, 0, 103, 0,
        117, 0, 108, 0, 97, 0, 114, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
}
// Exact structural-negative SFNT prefix generated from the pre-change metadata
// fixture. Its CFF program is deliberately invalid; no synthetic admission proof.
pub fn replace(source: &[u8], tag: [u8; 4], replacement: Option<Vec<u8>>) -> Vec<u8> {
    let count = usize::from(u16::from_be_bytes([source[4], source[5]]));
    let mut tables = std::collections::BTreeMap::new();
    for r in source[12..12 + count * 16].chunks_exact(16) {
        let t: [u8; 4] = r[..4].try_into().unwrap();
        let at = u32::from_be_bytes(r[8..12].try_into().unwrap()) as usize;
        let n = u32::from_be_bytes(r[12..16].try_into().unwrap()) as usize;
        let mut b = source[at..at + n].to_vec();
        if t == *b"head" {
            b[8..12].fill(0);
        }
        tables.insert(t, b);
    }
    if let Some(data) = replacement {
        tables.insert(tag, data);
    } else {
        tables.remove(&tag);
    }
    let count = tables.len();
    let power = 1usize << (usize::BITS - 1 - count.leading_zeros());
    let mut b = vec![0u8; 12 + count * 16];
    b[..4].copy_from_slice(b"OTTO");
    b[4..6].copy_from_slice(&(count as u16).to_be_bytes());
    b[6..8].copy_from_slice(&((power * 16) as u16).to_be_bytes());
    b[8..10].copy_from_slice(&(power.trailing_zeros() as u16).to_be_bytes());
    b[10..12].copy_from_slice(&((count * 16 - power * 16) as u16).to_be_bytes());
    let mut head = 0;
    for (i, (tag, data)) in tables.into_iter().enumerate() {
        let at = 12 + i * 16;
        let start = b.len();
        b[at..at + 4].copy_from_slice(&tag);
        b[at + 4..at + 8].copy_from_slice(&checksum(&data).to_be_bytes());
        b[at + 8..at + 12].copy_from_slice(&(start as u32).to_be_bytes());
        b[at + 12..at + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        if tag == *b"head" {
            head = start;
        }
        b.extend(data);
        while b.len() % 4 != 0 {
            b.push(0);
        }
    }
    let sum = checksum(&b);
    b[head + 8..head + 12].copy_from_slice(&0xb1b0_afbau32.wrapping_sub(sum).to_be_bytes());
    b
}
fn checksum(b: &[u8]) -> u32 {
    b.chunks(4).fold(0u32, |s, c| {
        let mut w = [0; 4];
        w[..c.len()].copy_from_slice(c);
        s.wrapping_add(u32::from_be_bytes(w))
    })
}
pub type VerticalCase = (u16, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>);
pub fn vertical_cases() -> Vec<VerticalCase> {
    let mut h = vec![0; 36];
    h[..4].copy_from_slice(&0x10000u32.to_be_bytes());
    h[34..].copy_from_slice(&1u16.to_be_bytes());
    let mut v = vec![0, 1, 0, 0, 0, 0, 0, 2];
    v.extend([0, 0, 0, 10, 0, 3, 255, 250]);
    let m = vec![0; 10];
    let mut cases = vec![
        (4, None, None, None),
        (0, None, None, None),
        (4, Some(v.clone()), Some(h.clone()), Some(m.clone())),
    ];
    for which in 0..3 {
        let base = match which {
            0 => &v,
            1 => &h,
            _ => &m,
        };
        for end in 0..base.len() {
            let mut c = (4, Some(v.clone()), Some(h.clone()), Some(m.clone()));
            match which {
                0 => c.1 = Some(base[..end].to_vec()),
                1 => c.2 = Some(base[..end].to_vec()),
                _ => c.3 = Some(base[..end].to_vec()),
            }
            cases.push(c);
        }
        for at in 0..base.len() {
            for value in [0, 1, 255] {
                if value == base[at] {
                    continue;
                }
                let mut data = base.clone();
                data[at] = value;
                let mut c = (4, Some(v.clone()), Some(h.clone()), Some(m.clone()));
                match which {
                    0 => c.1 = Some(data),
                    1 => c.2 = Some(data),
                    _ => c.3 = Some(data),
                }
                cases.push(c);
            }
        }
    }
    for a in [None, Some(v.clone())] {
        for b in [None, Some(h.clone())] {
            for c in [None, Some(m.clone())] {
                cases.push((4, a.clone(), b.clone(), c));
            }
        }
    }
    cases
}
pub fn cases() -> Vec<(String, Vec<u8>)> {
    let good = standard();
    let mut cases = vec![("standard".into(), good.clone())];
    let count = usize::from(u16::from_be_bytes([good[4], good[5]]));
    for tag in [*b"head", *b"maxp", *b"hhea", *b"OS/2", *b"post"] {
        let rec = good[12..12 + count * 16]
            .chunks_exact(16)
            .find(|r| r[..4] == tag)
            .unwrap();
        let at = u32::from_be_bytes(rec[8..12].try_into().unwrap()) as usize;
        let n = u32::from_be_bytes(rec[12..16].try_into().unwrap()) as usize;
        let bytes = &good[at..at + n];
        for end in 0..=n {
            cases.push((
                format!("{tag:?}-len-{end}"),
                replace(&good, tag, Some(bytes[..end].to_vec())),
            ));
        }
        for i in 0..n {
            for v in [0, 1, 255] {
                if bytes[i] == v {
                    continue;
                }
                let mut data = bytes.to_vec();
                data[i] = v;
                cases.push((
                    format!("{tag:?}-byte-{i}-{v}"),
                    replace(&good, tag, Some(data)),
                ));
            }
        }
    }
    for (i, (glyphs, v, h, m)) in vertical_cases().into_iter().enumerate() {
        if glyphs != 4 {
            continue;
        }
        let mut b = good.clone();
        for (tag, data) in [(*b"VORG", v), (*b"vhea", h), (*b"vmtx", m)] {
            b = replace(&b, tag, data);
        }
        cases.push((format!("vertical-{i}"), b));
    }
    cases
}
