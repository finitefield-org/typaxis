// Test-only AAT tables over unchanged base outlines, cmap and metrics.
// The resulting font is a derived fixture, not an original-book font receipt.
use std::{vec, vec::Vec};

pub fn aat_tables(chains: u32, lower_case: bool) -> Vec<([u8; 4], Vec<u8>)> {
    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|n| n.to_be_bytes()).collect()
    }
    fn longs(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|n| n.to_be_bytes()).collect()
    }
    assert!(chains > 0 && chains <= 16);
    let kinds: &[u16] = if lower_case {
        &[1, 3, 17, 37]
    } else {
        &[1, 3, 17]
    };
    let mut feat = longs(&[0x0001_0000]);
    feat.extend(words(&[kinds.len() as u16, 0, 0, 0]));
    let settings_start = 12 + 12 * kinds.len();
    for (index, kind) in kinds.iter().enumerate() {
        feat.extend(words(&[*kind, 6]));
        feat.extend(longs(&[(settings_start + 24 * index) as u32]));
        feat.extend(words(&[
            if *kind == 1 { 0 } else { 0x8000 },
            256 + index as u16,
        ]));
    }
    for _ in kinds {
        for setting in 0..6 {
            feat.extend(words(&[setting, 260 + setting]));
        }
    }
    let mut morx = longs(&[0x0002_0000, chains]);
    for _ in 0..chains {
        // One non-contextual lookup replaces glyph 18 ('1') with 19 ('2')
        // in both repository base fonts. The outlines themselves are retained.
        morx.extend(longs(&[1, 96, 5, 1]));
        for (kind, setting, enable, disable) in [
            (1, 2, 1, 0xffff_ffff),
            (1, 3, 0, 0xffff_fffe),
            (3, 3, 2, 0xffff_ffff),
            (17, 1, 4, 0xffff_ffff),
            (37, 1, 8, 0xffff_ffff),
        ] {
            morx.extend(words(&[kind, setting]));
            morx.extend(longs(&[enable, disable]));
        }
        morx.extend(longs(&[20, 0x2000_0004, 1]));
        morx.extend(words(&[8, 18, 1, 19]));
    }
    vec![(*b"feat", feat), (*b"morx", morx)]
}
