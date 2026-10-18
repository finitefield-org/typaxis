// A derived test font with one explicit DFLT/ccmp multiple substitution.
// This fixture builder is test-only and does not issue font-admission receipts.
use std::vec::Vec;

pub fn multiple_substitution_font(base: &[u8], glyph: u16, count: u16) -> Vec<u8> {
    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|n| n.to_be_bytes()).collect()
    }
    assert!(count > 1);
    let mut gsub = words(&[1, 0, 10, 30, 44]);
    gsub.extend(words(&[1]));
    gsub.extend(b"DFLT");
    gsub.extend(words(&[8, 4, 0, 0, 0xffff, 1, 0]));
    gsub.extend(words(&[1]));
    gsub.extend(b"ccmp");
    gsub.extend(words(&[8, 0, 1, 0]));
    gsub.extend(words(&[1, 4, 2, 0, 1, 8, 1, 8, 1, 14, 1, 1, glyph, count]));
    gsub.extend((0..count).flat_map(|_| glyph.to_be_bytes()));
    with_replaced_tables(base, &[(*b"GSUB", gsub)])
}

pub fn with_replaced_tables(base: &[u8], replacements: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    fn u16_at(bytes: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap())
    }
    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }
    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|n| n.to_be_bytes()).collect()
    }
    fn checksum(bytes: &[u8]) -> u32 {
        bytes.chunks(4).fold(0u32, |sum, chunk| {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            sum.wrapping_add(u32::from_be_bytes(word))
        })
    }
    let mut tables: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    for index in 0..usize::from(u16_at(base, 4)) {
        let record = 12 + 16 * index;
        let tag: [u8; 4] = base[record..record + 4].try_into().unwrap();
        if replacements.iter().any(|row| row.0 == tag) {
            continue;
        }
        let start = u32_at(base, record + 8) as usize;
        let length = u32_at(base, record + 12) as usize;
        let mut bytes = base[start..start + length].to_vec();
        if &tag == b"head" {
            bytes[8..12].fill(0);
        }
        tables.push((tag, bytes));
    }
    tables.extend_from_slice(replacements);
    tables.sort_by_key(|row| row.0);
    let n = u16::try_from(tables.len()).unwrap();
    let mut power = 1u16;
    let mut selector = 0u16;
    while power * 2 <= n {
        power *= 2;
        selector += 1;
    }
    let mut output = base[..4].to_vec();
    output.extend(words(&[n, power * 16, selector, n * 16 - power * 16]));
    let mut offset = 12 + 16 * tables.len();
    let mut head = None;
    for (tag, bytes) in &tables {
        output.extend(tag);
        output.extend(checksum(bytes).to_be_bytes());
        output.extend((offset as u32).to_be_bytes());
        output.extend((bytes.len() as u32).to_be_bytes());
        if tag == b"head" {
            head = Some(offset);
        }
        offset += (bytes.len() + 3) & !3;
    }
    for (_, bytes) in tables {
        output.extend(bytes);
        while output.len() % 4 != 0 {
            output.push(0);
        }
    }
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&output));
    let head = head.unwrap();
    output[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    assert_eq!(checksum(&output), 0xB1B0_AFBA);
    output
}
