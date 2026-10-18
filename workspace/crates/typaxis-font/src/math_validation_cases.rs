#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub glyphs: u16,
    pub bytes: Vec<u8>,
}
fn words(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|n| n.to_be_bytes()).collect()
}
fn put(b: &mut [u8], at: usize, n: usize) {
    b[at..at + 2].copy_from_slice(&u16::try_from(n).unwrap().to_be_bytes());
}
fn child(b: &mut Vec<u8>, field: usize, base: usize, data: Vec<u8>) -> usize {
    let at = b.len();
    put(b, field, at - base);
    b.extend(data);
    at
}
fn coverage(range: bool) -> Vec<u8> {
    if range {
        words(&[2, 1, 0, 1, 0])
    } else {
        words(&[1, 2, 0, 2])
    }
}
fn value_map(range: bool) -> Vec<u8> {
    let mut b = words(&[0, 2, 20, 0, 30, 0]);
    child(&mut b, 0, 0, coverage(range));
    b
}
fn construction() -> Vec<u8> {
    let mut b = words(&[0, 2, 1, 900, 2, 1400]);
    child(
        &mut b,
        0,
        0,
        words(&[12, 0, 2, 0, 0, 0, 500, 0, 3, 100, 120, 600, 1]),
    );
    b
}
fn minimal() -> Vec<u8> {
    let mut b = words(&[1, 0, 10, 224, 232]);
    let mut constants = vec![0; 214];
    put(&mut constants, 0, 70);
    put(&mut constants, 2, 50);
    put(&mut constants, 6, 1300);
    for i in 0..51 {
        put(&mut constants, 8 + i * 4, 1);
    }
    put(&mut constants, 8 + 34 * 4, 40);
    put(&mut constants, 8 + 47 * 4, 40);
    b.extend(constants);
    b.extend([0; 18]);
    b
}
fn nested(shared: bool) -> Vec<u8> {
    let mut b = minimal();
    b.truncate(232);
    child(&mut b, 224, 224, value_map(false));
    child(&mut b, 226, 224, value_map(true));
    child(&mut b, 228, 224, coverage(false));
    let variants = b.len();
    put(&mut b, 8, variants);
    b.extend(words(&[10, 0, 0, 2, 2, 0, 0, 0, 0]));
    child(&mut b, variants + 2, variants, coverage(false));
    child(&mut b, variants + 4, variants, coverage(true));
    let first = child(&mut b, variants + 10, variants, construction());
    let second = child(&mut b, variants + 12, variants, construction());
    if shared {
        put(&mut b, variants + 14, first - variants);
        put(&mut b, variants + 16, second - variants);
    } else {
        child(&mut b, variants + 14, variants, construction());
        child(&mut b, variants + 16, variants, construction());
    }
    b
}
pub fn font_case(font: &[u8]) -> Case {
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    let table = |tag: [u8; 4]| {
        let r = font[12..12 + count * 16]
            .chunks_exact(16)
            .find(|r| r[..4] == tag)
            .unwrap();
        let a = u32::from_be_bytes(r[8..12].try_into().unwrap()) as usize;
        let n = u32::from_be_bytes(r[12..16].try_into().unwrap()) as usize;
        &font[a..a + n]
    };
    let maxp = table(*b"maxp");
    Case {
        label: "existing-math-fixture".into(),
        glyphs: u16::from_be_bytes([maxp[4], maxp[5]]),
        bytes: table(*b"MATH").to_vec(),
    }
}
pub fn controls() -> Vec<Case> {
    [
        ("minimal", minimal()),
        ("nested", nested(false)),
        ("shared-constructions", nested(true)),
    ]
    .into_iter()
    .map(|(label, bytes)| Case {
        label: label.into(),
        glyphs: 4,
        bytes,
    })
    .collect()
}
pub fn cases(font: &[u8]) -> Vec<Case> {
    let mut good = controls();
    good.push(font_case(font));
    let mut out = Vec::new();
    for c in good {
        out.push(c.clone());
        for end in 0..c.bytes.len() {
            let mut v = c.clone();
            v.label = format!("{}-prefix-{end}", c.label);
            v.bytes.truncate(end);
            out.push(v);
        }
        for at in 0..c.bytes.len() {
            for value in [0, 1, 2, 128, 255] {
                if c.bytes[at] == value {
                    continue;
                }
                let mut v = c.clone();
                v.label = format!("{}-byte-{at}-{value}", c.label);
                v.bytes[at] = value;
                out.push(v);
            }
        }
        for glyphs in [0, 1, 2, 3, 5, u16::MAX] {
            let mut v = c.clone();
            v.label = format!("{}-glyphs-{glyphs}", c.label);
            v.glyphs = glyphs;
            out.push(v);
        }
        for extra in [1, 2, 8] {
            let mut v = c.clone();
            v.label = format!("{}-trailing-{extra}", c.label);
            v.bytes.resize(v.bytes.len() + extra, 0);
            out.push(v);
        }
    }
    out
}
