// Derived OpenType fixtures. Outlines/cmap/metrics are copied from the caller's
// base font; these tables are test inputs and issue no admission receipts.
use std::vec::Vec;

pub struct Context<'a> {
    pub format: u16,
    pub chained: bool,
    pub input: &'a [u16],
    pub nested_index: u16,
    pub growth: u16,
    pub positioning: bool,
    pub ignore_marks: bool,
}
impl<'a> Context<'a> {
    pub fn sequence(format: u16, chained: bool, input: &'a [u16], growth: u16) -> Self {
        Self {
            format,
            chained,
            input,
            nested_index: 0,
            growth,
            positioning: false,
            ignore_marks: false,
        }
    }
}
fn words(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}
fn append(target: &mut Vec<u8>, bytes: &[u8]) -> u16 {
    let offset = u16::try_from(target.len()).unwrap();
    target.extend(bytes);
    offset
}
fn put(target: &mut [u8], word: usize, value: u16) {
    target[2 * word..2 * word + 2].copy_from_slice(&value.to_be_bytes());
}
fn coverage(glyph: u16) -> Vec<u8> {
    words(&[1, 1, glyph])
}
fn lookup(kind: u16, flags: u16, subtable: Vec<u8>) -> Vec<u8> {
    let mut result = words(&[kind, flags, 1, 8]);
    result.extend(subtable);
    result
}
fn layout(tag: &[u8; 4], lookups: &[Vec<u8>]) -> Vec<u8> {
    let mut result = words(&[1, 0, 10, 30, 44, 1]);
    result.extend(b"DFLT");
    result.extend(words(&[8, 4, 0, 0, 0xffff, 1, 0, 1]));
    result.extend(tag);
    result.extend(words(&[8, 0, 1, 0]));
    let mut list = words(&[lookups.len() as u16]);
    list.resize(2 + lookups.len() * 2, 0);
    for (i, table) in lookups.iter().enumerate() {
        let offset = append(&mut list, table);
        put(&mut list, i + 1, offset);
    }
    result.extend(list);
    result
}
fn single(glyph: u16, replacement: u16, positioning: bool) -> Vec<u8> {
    let subtable = if positioning {
        let mut subtable = words(&[1, 8, 4, replacement]);
        subtable.extend(coverage(glyph));
        subtable
    } else {
        let mut subtable = words(&[2, 8, 1, replacement]);
        subtable.extend(coverage(glyph));
        subtable
    };
    lookup(1, 0, subtable)
}
fn multiple(glyph: u16, count: u16) -> Vec<u8> {
    let mut subtable = words(&[1, 0, 1, 8, count]);
    subtable.extend((0..count).flat_map(|_| 19u16.to_be_bytes()));
    let offset = append(&mut subtable, &coverage(glyph));
    put(&mut subtable, 1, offset);
    lookup(2, 0, subtable)
}
fn gdef() -> Vec<u8> {
    // The ASCII 'a' glyph (66) is a mark for IgnoreMarks fixtures.
    words(&[1, 0, 12, 0, 0, 0, 2, 2, 18, 20, 1, 66, 66, 3])
}
pub fn contextual_tables(config: &Context<'_>) -> Vec<([u8; 4], Vec<u8>)> {
    let n = config.input.len();
    assert!((1..=3).contains(&config.format) && n > 1);
    assert!(usize::from(config.nested_index) < n && config.growth > 0);
    let records = words(&[config.nested_index, 1, n as u16 - 1, 2]);
    let mut classes = config.input.to_vec();
    classes.extend([17, 21]);
    classes.sort_unstable();
    classes.dedup();
    let class = |glyph| (classes.binary_search(&glyph).unwrap() + 1) as u16;
    let mut definition = words(&[2, classes.len() as u16]);
    for glyph in &classes {
        definition.extend(words(&[*glyph, *glyph, class(*glyph)]));
    }
    let mut subtable;
    if config.format == 3 {
        let mut values = vec![3];
        let mut slots = Vec::new();
        let mut glyphs = Vec::new();
        if config.chained {
            values.push(1);
            slots.push(values.len());
            values.push(0);
            glyphs.push(17);
        }
        values.push(n as u16);
        if !config.chained {
            values.push(2);
        }
        for glyph in config.input {
            slots.push(values.len());
            values.push(0);
            glyphs.push(*glyph);
        }
        if config.chained {
            values.push(1);
            slots.push(values.len());
            values.push(0);
            glyphs.push(21);
            values.push(2);
        }
        subtable = words(&values);
        subtable.extend(&records);
        for (slot, glyph) in slots.into_iter().zip(glyphs) {
            let offset = append(&mut subtable, &coverage(glyph));
            put(&mut subtable, slot, offset);
        }
    } else {
        let mut rule = Vec::new();
        let convert = |glyph| {
            if config.format == 2 {
                class(glyph)
            } else {
                glyph
            }
        };
        if config.chained {
            rule.extend(words(&[1, convert(17)]));
        }
        rule.extend(words(&[n as u16]));
        if !config.chained {
            rule.extend(words(&[2]));
        }
        for glyph in &config.input[1..] {
            rule.extend(words(&[convert(*glyph)]));
        }
        if config.chained {
            rule.extend(words(&[1, convert(21), 2]));
        }
        rule.extend(&records);
        let mut set = words(&[1, 4]);
        set.extend(rule);
        if config.format == 1 {
            subtable = words(&[1, 0, 1, 8]);
            subtable.extend(set);
        } else {
            let set_count = classes.len() + 1;
            let head_words = if config.chained { 6 } else { 4 };
            let mut header = vec![2, 0];
            if config.chained {
                header.extend([0, 0, 0]);
            } else {
                header.push(0);
            }
            header.push(set_count as u16);
            header.resize(head_words + set_count, 0);
            subtable = words(&header);
            let offset = append(&mut subtable, &set);
            put(
                &mut subtable,
                head_words + class(config.input[0]) as usize,
                offset,
            );
            let offset = append(&mut subtable, &definition);
            if config.chained {
                for word in 2..5 {
                    put(&mut subtable, word, offset);
                }
            } else {
                put(&mut subtable, 2, offset);
            }
        }
        let offset = append(&mut subtable, &coverage(config.input[0]));
        put(&mut subtable, 1, offset);
    }
    let nested_glyph = config.input[config.nested_index as usize];
    let nested = if config.positioning {
        single(nested_glyph, 100, true)
    } else if config.growth == 1 {
        single(nested_glyph, 19, false)
    } else {
        multiple(nested_glyph, config.growth)
    };
    let kind = match (config.positioning, config.chained) {
        (false, false) => 5,
        (false, true) => 6,
        (true, false) => 7,
        (true, true) => 8,
    };
    let table = layout(
        if config.positioning { b"kern" } else { b"ccmp" },
        &[
            lookup(kind, if config.ignore_marks { 8 } else { 0 }, subtable),
            nested,
            // The original recursive kernel indexes the updated position list.
            // After expansion this record addresses an inserted glyph.
            single(
                if !config.positioning && config.growth > 1 {
                    19
                } else {
                    *config.input.last().unwrap()
                },
                if config.positioning { 50 } else { 20 },
                config.positioning,
            ),
        ],
    );
    let mut result = vec![(
        if config.positioning {
            *b"GPOS"
        } else {
            *b"GSUB"
        },
        table,
    )];
    if config.ignore_marks {
        result.push((*b"GDEF", gdef()));
    }
    result
}
pub fn ligature_tables(count: u16, ignore_marks: bool) -> Vec<([u8; 4], Vec<u8>)> {
    assert!(count > 1);
    let mut subtable = words(&[1, 0, 1, 8, 1, 4, 19, count]);
    subtable.extend((1..count).flat_map(|_| 18u16.to_be_bytes()));
    let offset = append(&mut subtable, &coverage(18));
    put(&mut subtable, 1, offset);
    let mut result = vec![(
        *b"GSUB",
        layout(
            b"liga",
            &[lookup(4, if ignore_marks { 8 } else { 0 }, subtable)],
        ),
    )];
    if ignore_marks {
        result.push((*b"GDEF", gdef()));
    }
    result
}
