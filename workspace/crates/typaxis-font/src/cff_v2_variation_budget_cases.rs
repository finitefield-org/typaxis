pub fn table(selectors: usize, shared: bool, reverse: bool) -> Vec<u8> {
    let mut b = vec![0u8; 10 + 11 * selectors];
    b[..2].copy_from_slice(&14u16.to_be_bytes());
    b[6..10].copy_from_slice(&(selectors as u32).to_be_bytes());
    let mut offsets = Vec::new();
    for _ in 0..if shared {
        usize::from(selectors != 0)
    } else {
        selectors
    } {
        let default = b.len() as u32;
        b.extend(1u32.to_be_bytes());
        b.extend([0, 0x4e, 0, 2]);
        let non = b.len() as u32;
        b.extend(1u32.to_be_bytes());
        b.extend([0, 0x4e, 3, 0, 4]);
        offsets.push((default, non));
    }
    for i in 0..selectors {
        let selector = if selectors == 256 && i < 16 {
            0xfe00 + i as u32
        } else {
            0xe0100 + (i - if selectors == 256 { 16 } else { 0 }) as u32
        };
        let at = 10 + 11 * i;
        b[at..at + 3].copy_from_slice(&selector.to_be_bytes()[1..]);
        let j = if shared {
            0
        } else if reverse {
            selectors - 1 - i
        } else {
            i
        };
        b[at + 3..at + 7].copy_from_slice(&offsets[j].0.to_be_bytes());
        b[at + 7..at + 11].copy_from_slice(&offsets[j].1.to_be_bytes());
    }
    let len = b.len() as u32;
    b[2..6].copy_from_slice(&len.to_be_bytes());
    b
}
pub fn cmap(table: &[u8], base: bool) -> Vec<u8> {
    let records = if base { 2u16 } else { 1 };
    let mut b = vec![0, 0];
    b.extend(records.to_be_bytes());
    if base {
        b.extend([0, 3, 0, 10]);
        b.extend(20u32.to_be_bytes());
    }
    b.extend([0, 0, 0, 5]);
    b.extend((if base { 48u32 } else { 12 }).to_be_bytes());
    if base {
        b.extend([0, 12, 0, 0]);
        for n in [28u32, 0, 1, 0x4e00, 0x4e05, 1] {
            b.extend(n.to_be_bytes());
        }
    }
    b.extend(table);
    b
}
pub fn cases() -> Vec<(String, Vec<u8>, u16)> {
    let mut out = Vec::new();
    for (n, shared, reverse) in [
        (0, false, false),
        (1, false, false),
        (2, true, false),
        (2, false, true),
        (3, false, true),
        (8, true, false),
        (17, false, true),
        (256, true, false),
    ] {
        let t = table(n, shared, reverse);
        for base in [false, true] {
            out.push((
                format!("selectors-{n}-shared-{shared}-reverse-{reverse}-base-{base}"),
                cmap(&t, base),
                8,
            ));
        }
    }
    let valid = table(1, false, false);
    for (label, at, bytes) in [
        ("intersect", 35, vec![1]),
        ("gid", 37, vec![8]),
        ("surrogate", 25, vec![0, 0xd7, 0xff]),
        ("selector", 10, vec![0, 0, 65]),
        ("overlap", 13, 10u32.to_be_bytes().to_vec()),
        ("no-offsets", 13, vec![0; 8]),
        ("selector-limit", 6, 257u32.to_be_bytes().to_vec()),
        ("zero-gid", 36, vec![0, 0]),
        ("no-defaults", 13, vec![0; 4]),
        ("no-nondefaults", 17, vec![0; 4]),
    ] {
        let mut t = valid.clone();
        t[at..at + bytes.len()].copy_from_slice(&bytes);
        out.push((label.into(), cmap(&t, true), 8));
    }
    let mut overlap = table(2, true, false);
    overlap[24..28].copy_from_slice(&40u32.to_be_bytes());
    overlap[28..32].fill(0);
    out.push(("cross-kind-alias".into(), cmap(&overlap, true), 8));
    out.push(("zero-glyphs".into(), cmap(&valid, true), 0));
    out.push(("no-variation".into(), vec![0, 0, 0, 0], 8));
    out
}
