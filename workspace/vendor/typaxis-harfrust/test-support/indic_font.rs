// Derived Indic fixtures: reuse caller outlines/metrics, preserve its cmap, and
// add explicit Indic cmap/script entries. These are not original book fonts.
use std::collections::BTreeMap;
use std::vec::Vec;
fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
}
fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap())
}
fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|n| n.to_be_bytes()).collect()
}
fn original_cmap(base: &[u8]) -> BTreeMap<u32, u16> {
    let table = (0..usize::from(u16_at(base, 4)))
        .find_map(|index| {
            let at = 12 + 16 * index;
            (&base[at..at + 4] == b"cmap").then(|| u32_at(base, at + 8) as usize)
        })
        .unwrap();
    let offsets = (0..usize::from(u16_at(base, table + 2)))
        .map(|i| table + u32_at(base, table + 8 + 8 * i) as usize)
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    if let Some(offset) = offsets.iter().copied().find(|o| u16_at(base, *o) == 12) {
        for i in 0..u32_at(base, offset + 12) as usize {
            let group = offset + 16 + 12 * i;
            let start = u32_at(base, group);
            let end = u32_at(base, group + 4);
            let glyph = u32_at(base, group + 8);
            for codepoint in start..=end {
                result.insert(codepoint, u16::try_from(glyph + codepoint - start).unwrap());
            }
        }
    } else {
        let offset = offsets
            .iter()
            .copied()
            .find(|o| u16_at(base, *o) == 4)
            .unwrap();
        let n = usize::from(u16_at(base, offset + 6) / 2);
        for i in 0..n {
            let end = u16_at(base, offset + 14 + 2 * i);
            let start = u16_at(base, offset + 16 + 2 * n + 2 * i);
            if start == 0xffff {
                continue;
            }
            let delta = u16_at(base, offset + 16 + 4 * n + 2 * i);
            let range_at = offset + 16 + 6 * n + 2 * i;
            let range = u16_at(base, range_at);
            for codepoint in start..=end {
                let glyph = if range == 0 {
                    codepoint.wrapping_add(delta)
                } else {
                    let raw = u16_at(
                        base,
                        range_at + usize::from(range) + 2 * usize::from(codepoint - start),
                    );
                    if raw == 0 {
                        0
                    } else {
                        raw.wrapping_add(delta)
                    }
                };
                if glyph != 0 {
                    result.insert(u32::from(codepoint), glyph);
                }
            }
        }
    }
    result
}
pub fn indic_tables(base: &[u8], old_spec: bool) -> Vec<([u8; 4], Vec<u8>)> {
    let tags = if old_spec {
        [
            *b"beng", *b"deva", *b"gujr", *b"guru", *b"knda", *b"mlym", *b"orya", *b"taml",
            *b"telu",
        ]
    } else {
        [
            *b"bng2", *b"dev2", *b"gjr2", *b"gur2", *b"knd2", *b"mlm2", *b"ory2", *b"tel2",
            *b"tml2",
        ]
    };
    script_tables(base, 0x0900..=0x0dff, &tags)
}

pub fn script_tables(
    base: &[u8],
    codepoints: core::ops::RangeInclusive<u32>,
    tags: &[[u8; 4]],
) -> Vec<([u8; 4], Vec<u8>)> {
    let mut mappings = original_cmap(base);
    let first = *codepoints.start();
    for codepoint in codepoints {
        mappings.insert(codepoint, 2 + ((codepoint - first) % 94) as u16);
    }
    mappings.insert(0x200c, 1);
    mappings.insert(0x200d, 1);
    mappings.insert(0x25cc, 95);
    let mut cmap = words(&[0, 1, 3, 10]);
    cmap.extend(12u32.to_be_bytes());
    cmap.extend(words(&[12, 0]));
    cmap.extend((16 + 12 * mappings.len() as u32).to_be_bytes());
    cmap.extend(0u32.to_be_bytes());
    cmap.extend((mappings.len() as u32).to_be_bytes());
    for (codepoint, glyph) in mappings {
        cmap.extend(codepoint.to_be_bytes());
        cmap.extend(codepoint.to_be_bytes());
        cmap.extend(u32::from(glyph).to_be_bytes());
    }
    let mut scripts = words(&[tags.len() as u16]);
    for (i, tag) in tags.iter().enumerate() {
        scripts.extend(tag);
        scripts.extend(((2 + 6 * tags.len() + 10 * i) as u16).to_be_bytes());
    }
    for _ in tags {
        scripts.extend(words(&[4, 0, 0, 0xffff, 0]));
    }
    let features = 10 + scripts.len() as u16;
    let lookups = features + 2;
    let mut gsub = words(&[1, 0, 10, features, lookups]);
    gsub.extend(scripts);
    gsub.extend(words(&[0, 0]));
    vec![(*b"cmap", cmap), (*b"GSUB", gsub)]
}
