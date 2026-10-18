pub fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, c| {
        let mut word = [0u8; 4];
        word[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}
pub fn fixture(length: usize, reverse: bool, gap: usize, vertical: bool) -> Vec<u8> {
    let mut tags = vec![
        *b"CFF ", *b"OS/2", *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"post",
    ];
    if vertical {
        tags.push(*b"VORG");
        tags.sort_unstable();
    }
    let count = tags.len();
    let mut bytes = vec![0u8; 12 + count * 16];
    bytes[..4].copy_from_slice(b"OTTO");
    bytes[4..6].copy_from_slice(&(count as u16).to_be_bytes());
    let power = 1usize << (usize::BITS - 1 - count.leading_zeros());
    bytes[6..8].copy_from_slice(&((power * 16) as u16).to_be_bytes());
    bytes[8..10].copy_from_slice(&(power.trailing_zeros() as u16).to_be_bytes());
    bytes[10..12].copy_from_slice(&((count * 16 - power * 16) as u16).to_be_bytes());
    let mut order = (0..count).collect::<Vec<_>>();
    if reverse {
        order.reverse();
    }
    let mut head = 0;
    for i in order {
        bytes.resize(bytes.len() + gap * 4, 0);
        let start = bytes.len();
        let size = if tags[i] == *b"head" { 12 } else { length };
        bytes.resize(start + size, 0);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        let p = 12 + i * 16;
        bytes[p..p + 4].copy_from_slice(&tags[i]);
        bytes[p + 8..p + 12].copy_from_slice(&(start as u32).to_be_bytes());
        bytes[p + 12..p + 16].copy_from_slice(&(size as u32).to_be_bytes());
        if tags[i] == *b"head" {
            head = start;
        }
    }
    let sum = checksum(&bytes);
    bytes[head + 8..head + 12].copy_from_slice(&0xb1b0_afbau32.wrapping_sub(sum).to_be_bytes());
    bytes
}
pub fn cases() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (length, reverse, gap, vertical) in [
        (1, false, 0, false),
        (3, true, 1, false),
        (4, false, 0, false),
        (5, true, 2, false),
        (63, true, 0, false),
        (64, false, 0, false),
        (65, true, 1, false),
        (255, true, 0, false),
        (256, false, 0, false),
        (257, true, 0, false),
        (4, true, 0, true),
    ] {
        out.push((
            format!("length-{length}-reverse-{reverse}-gap-{gap}-vertical-{vertical}"),
            fixture(length, reverse, gap, vertical),
        ));
    }
    let good = fixture(4, false, 0, false);
    let mut changed = good.clone();
    changed[28..32].copy_from_slice(b"CFF ");
    out.push(("duplicate-tag".into(), changed));
    let mut changed = good.clone();
    changed[12..16].copy_from_slice(b"ZZZZ");
    out.push(("unsupported-tag".into(), changed));
    let mut changed = good.clone();
    let offset = changed[20..24].to_vec();
    changed[36..40].copy_from_slice(&offset);
    out.push(("overlapping-offsets".into(), changed));
    let mut changed = good.clone();
    changed[4..6].copy_from_slice(&257u16.to_be_bytes());
    out.push(("too-many-tables".into(), changed));
    let mut changed = good.clone();
    changed[12 + 3 * 16 + 12..12 + 3 * 16 + 16].copy_from_slice(&8u32.to_be_bytes());
    out.push(("short-head".into(), changed));
    out
}
