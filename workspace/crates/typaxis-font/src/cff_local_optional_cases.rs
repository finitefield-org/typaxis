#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub tag: [u8; 4],
    pub glyphs: u16,
    pub bytes: Vec<u8>,
}
fn words(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|n| n.to_be_bytes()).collect()
}
fn put(b: &mut [u8], at: usize, value: usize) {
    b[at..at + 2].copy_from_slice(&u16::try_from(value).unwrap().to_be_bytes());
}
fn child(b: &mut Vec<u8>, field: usize, base: usize, data: Vec<u8>) -> usize {
    let at = b.len();
    put(b, field, at - base);
    b.extend(data);
    at
}
fn coord(format: u16) -> Vec<u8> {
    match format {
        1 => words(&[1, 0]),
        2 => words(&[2, 0, 1, 0]),
        3 => words(&[3, 0, 6, 8, 12, 1, 0]),
        _ => unreachable!(),
    }
}
fn min_max() -> Vec<u8> {
    let mut b = words(&[0, 0, 1]);
    b.extend(*b"kern");
    b.extend([0; 4]);
    child(&mut b, 0, 0, coord(1));
    child(&mut b, 2, 0, coord(2));
    child(&mut b, 10, 0, coord(3));
    child(&mut b, 12, 0, coord(1));
    b
}
fn base_values() -> Vec<u8> {
    let mut b = words(&[1, 3, 0, 0, 0]);
    for i in 0..3 {
        child(&mut b, 4 + 2 * i, 0, coord(i as u16 + 1));
    }
    b
}
fn base_script() -> Vec<u8> {
    let mut b = words(&[0, 0, 1]);
    b.extend(*b"JAN ");
    b.extend([0; 2]);
    child(&mut b, 0, 0, base_values());
    child(&mut b, 2, 0, min_max());
    child(&mut b, 10, 0, min_max());
    b
}
fn base() -> Vec<u8> {
    let mut axis = words(&[0, 0]);
    let mut tags = words(&[2]);
    tags.extend(*b"hang");
    tags.extend(*b"romn");
    child(&mut axis, 0, 0, tags);
    let scripts = axis.len();
    let mut list = words(&[1]);
    list.extend(*b"latn");
    list.extend([0; 2]);
    child(&mut axis, 2, 0, list);
    child(&mut axis, scripts + 6, scripts, base_script());
    let mut b = words(&[1, 0, 8, 8]);
    b.extend(axis);
    b
}
fn coverage(range: bool) -> Vec<u8> {
    if range {
        words(&[2, 1, 0, 1, 0])
    } else {
        words(&[1, 2, 0, 2])
    }
}
fn attach() -> Vec<u8> {
    let mut b = words(&[0, 2, 0, 0]);
    child(&mut b, 0, 0, coverage(false));
    child(&mut b, 4, 0, words(&[1, 0]));
    child(&mut b, 6, 0, words(&[2, 1, 2]));
    b
}
fn ligature() -> Vec<u8> {
    let mut b = words(&[3, 0, 0, 0]);
    child(&mut b, 2, 0, words(&[1, 0]));
    child(&mut b, 4, 0, words(&[2, 0]));
    child(&mut b, 6, 0, coord(3));
    b
}
fn carets() -> Vec<u8> {
    let mut b = words(&[0, 2, 0, 0]);
    child(&mut b, 0, 0, coverage(true));
    child(&mut b, 4, 0, ligature());
    child(&mut b, 6, 0, ligature());
    b
}
fn sets() -> Vec<u8> {
    let mut b = words(&[1, 2]);
    b.extend([0; 8]);
    for i in 0..2 {
        let at = b.len();
        b[4 + i * 4..8 + i * 4].copy_from_slice(&(at as u32).to_be_bytes());
        b.extend(coverage(i != 0));
    }
    b
}
fn gdef(v12: bool) -> Vec<u8> {
    let mut b = words(&[1, if v12 { 2 } else { 0 }, 0, 0, 0, 0]);
    if v12 {
        b.extend([0; 2]);
    }
    child(&mut b, 4, 0, words(&[1, 0, 3, 0, 1, 4]));
    child(&mut b, 6, 0, attach());
    child(&mut b, 8, 0, carets());
    child(&mut b, 10, 0, words(&[2, 2, 0, 1, 5, 2, 2, 9]));
    if v12 {
        child(&mut b, 12, 0, sets());
    }
    b
}
fn subkern(pairs: &[(u16, u16)], flags: u16) -> Vec<u8> {
    let count = pairs.len();
    let power = if count == 0 {
        0
    } else {
        1usize << (usize::BITS - 1 - count.leading_zeros())
    };
    let mut b = words(&[
        0,
        (14 + count * 6) as u16,
        flags,
        count as u16,
        (power * 6) as u16,
        if power == 0 {
            0
        } else {
            power.trailing_zeros() as u16
        },
        (count * 6 - power * 6) as u16,
    ]);
    for &(a, z) in pairs {
        b.extend(words(&[a, z, 0]));
    }
    b
}
fn kern() -> Vec<u8> {
    let mut b = words(&[0, 2]);
    b.extend(subkern(&[(0, 0), (0, 1), (2, 2)], 1));
    b.extend(subkern(&[], 15));
    b
}
pub fn controls() -> Vec<Case> {
    [
        ("base-empty", *b"BASE", words(&[1, 0, 0, 0])),
        ("base-nested-shared-axis", *b"BASE", base()),
        ("gdef-empty", *b"GDEF", words(&[1, 0, 0, 0, 0, 0])),
        ("gdef-10-nested", *b"GDEF", gdef(false)),
        ("gdef-12-nested", *b"GDEF", gdef(true)),
        ("kern-empty", *b"kern", words(&[0, 0])),
        ("kern-multi-subtable", *b"kern", kern()),
    ]
    .into_iter()
    .map(|(label, tag, bytes)| Case {
        label: label.into(),
        tag,
        glyphs: 4,
        bytes,
    })
    .collect()
}
pub fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for c in controls() {
        out.push(c.clone());
        for end in 0..c.bytes.len() {
            let mut row = c.clone();
            row.label = format!("{}-prefix-{end}", c.label);
            row.bytes.truncate(end);
            out.push(row);
        }
        for at in 0..c.bytes.len() {
            for value in [0, 1, 2, 3, 128, 255] {
                if c.bytes[at] == value {
                    continue;
                }
                let mut row = c.clone();
                row.label = format!("{}-byte-{at}-{value}", c.label);
                row.bytes[at] = value;
                out.push(row);
            }
        }
        for glyphs in [0, 1, 2, 3, 5, u16::MAX] {
            let mut row = c.clone();
            row.label = format!("{}-glyphs-{glyphs}", c.label);
            row.glyphs = glyphs;
            out.push(row);
        }
        for extra in [1, 2, 8] {
            let mut row = c.clone();
            row.label = format!("{}-trailing-{extra}", c.label);
            row.bytes.resize(row.bytes.len() + extra, 0);
            out.push(row);
        }
    }
    out
}
