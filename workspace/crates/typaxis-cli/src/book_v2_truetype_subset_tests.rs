use super::*;
use typaxis_resources::book_v2::{BookV2FontClosures, BookV2TrueTypeSubsetError as T};

pub(super) fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
pub(super) fn table<'a>(bytes: &'a [u8], face: u32, tag: &[u8; 4]) -> &'a [u8] {
    let offset = if bytes.starts_with(b"ttcf") {
        u32_at(bytes, 12 + face as usize * 4) as usize
    } else {
        assert_eq!(face, 0);
        0
    };
    let count = u16_at(bytes, offset + 4) as usize;
    let entry = (0..count)
        .map(|i| offset + 12 + 16 * i)
        .find(|i| &bytes[*i..*i + 4] == tag)
        .unwrap();
    let start = u32_at(bytes, entry + 8) as usize;
    let length = u32_at(bytes, entry + 12) as usize;
    &bytes[start..start + length]
}
pub(super) fn check(
    closures: &BookV2FontClosures<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let maximum = closures.work_steps().checked_add(10_000_000).unwrap();
    let display = closures.selection().display();
    let run = |records, spool, maximum_work, prior_work| -> Result<_, T> {
        let mut builder = Builder::new(display, limits, maximum_work, records, spool, prior_work)?;
        assert!(matches!(
            builder.write_truetype_font(closures, closures.fonts().len()),
            Err(T::InvalidFontIndex)
        ));
        let mut hashes = Vec::new();
        for (index, closed) in closures.fonts().iter().enumerate() {
            if closed.kind() == K::Cff1V2 {
                assert!(matches!(
                    builder.write_truetype_font(closures, index),
                    Err(T::WrongFontKind)
                ));
                continue;
            }
            let subset = builder.write_truetype_font(closures, index)?;
            assert!(std::ptr::eq(subset.source(), closed));
            assert_eq!(subset.record_charge(), builder.record_charge());
            assert_eq!(subset.spool_charge(), builder.spool_charge());
            assert_eq!(subset.work_steps(), builder.work_steps());
            let bytes = subset.bytes();
            assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
            assert_eq!(subset.sha256(), typaxis_core::sha256(bytes));
            assert!(bytes.len() as u64 <= limits.extension().get().max_font_subset_bytes);
            let checksum = bytes.chunks(4).fold(0u32, |sum, word| {
                let mut padded = [0; 4];
                padded[..word.len()].copy_from_slice(word);
                sum.wrapping_add(u32::from_be_bytes(padded))
            });
            assert_eq!(checksum, 0xb1b0_afba);
            assert_eq!(
                u16_at(table(bytes, 0, b"maxp"), 4) as usize,
                closed.glyphs().len()
            );
            assert_eq!(
                u16_at(table(bytes, 0, b"hhea"), 34) as usize,
                closed.glyphs().len()
            );
            let font = closed.source().instance().font();
            let original = font.bytes();
            let source_hhea = table(original, font.face_index(), b"hhea");
            let source_hmtx = table(original, font.face_index(), b"hmtx");
            let metric_count = u16_at(source_hhea, 34) as usize;
            let hmtx = table(bytes, 0, b"hmtx");
            assert_eq!(subset.original_widths().len(), closed.glyphs().len());
            for (dense, gid) in closed.glyphs().enumerate() {
                let index = gid.get() as usize;
                let advance = u16_at(source_hmtx, index.min(metric_count - 1) * 4);
                let bearing = u16_at(
                    source_hmtx,
                    if index < metric_count {
                        index * 4 + 2
                    } else {
                        metric_count * 4 + (index - metric_count) * 2
                    },
                );
                assert_eq!(subset.original_widths()[&gid], advance);
                assert_eq!(u16_at(hmtx, dense * 4), advance);
                assert_eq!(u16_at(hmtx, dense * 4 + 2), bearing);
                assert_eq!(closed.subset_gid(gid).unwrap().get() as usize, dense);
            }
            assert_eq!(
                u16_at(table(bytes, 0, b"head"), 18),
                font.metadata().units_per_em
            );
            let names = table(bytes, 0, b"name");
            assert_eq!(u16_at(names, 2), 1);
            assert_eq!(u16_at(names, 12), 6);
            let text = names[18..]
                .chunks_exact(2)
                .map(|p| u16::from_be_bytes(p.try_into().unwrap()))
                .collect::<Vec<_>>();
            assert_eq!(String::from_utf16(&text).unwrap(), subset.postscript_name());
            assert!(subset.postscript_name().ends_with("+Typaxis"));
            let loca = table(bytes, 0, b"loca");
            assert_eq!(u16_at(table(bytes, 0, b"head"), 50), 1);
            let offsets = loca
                .chunks_exact(4)
                .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
                .collect::<Vec<_>>();
            assert_eq!(offsets.len(), closed.glyphs().len() + 1);
            assert!(offsets.windows(2).all(|p| p[0] <= p[1]));
            assert_eq!(
                *offsets.last().unwrap() as usize,
                table(bytes, 0, b"glyf").len()
            );
            let again = builder.write_truetype_font(closures, index)?;
            assert_eq!(again.bytes(), bytes);
            assert_eq!(again.fingerprint(), subset.fingerprint());
            assert_eq!(again.metrics(), subset.metrics());
            hashes.push(subset.fingerprint());
        }
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            hashes,
        ))
    };
    let (records, spool, work, hashes) = run(0, 0, maximum, 0).unwrap();
    if hashes.is_empty() {
        return;
    }
    let baseline = (display.record_charge() + 1).max(closures.record_charge());
    let baseline_spool = display.source().spool_charge().max(closures.spool_charge());
    let prior = limits.base().get().max_fragments - (records - baseline) - 1;
    let prior_spool = limits.base().get().max_spool_bytes - (spool - baseline_spool);
    assert_eq!(
        run(prior, prior_spool, work, 0).unwrap(),
        (
            limits.base().get().max_fragments,
            limits.base().get().max_spool_bytes,
            work,
            hashes.clone()
        )
    );
    assert!(matches!(
        run(prior + 1, prior_spool, work, 0),
        Err(T::Budget(E::Records))
    ));
    assert!(matches!(
        run(prior, prior_spool + 1, work, 0),
        Err(T::Budget(E::Spool))
    ));
    assert!(matches!(run(0, 0, work - 1, 0), Err(T::Budget(E::Work))));
    assert_eq!(
        run(0, 0, work + 17, closures.work_steps() + 17).unwrap(),
        (records, spool, work + 17, hashes)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    let mut failed_at = None;
    'fonts: for (index, closed) in closures.fonts().iter().enumerate() {
        if closed.kind() != K::TrueType {
            continue;
        }
        for _ in 0..2 {
            match failed.write_truetype_font(closures, index) {
                Ok(_) => (),
                Err(T::Budget(E::Work)) => {
                    failed_at = Some(index);
                    break 'fonts;
                }
                Err(e) => panic!("unexpected subset failure: {e}"),
            }
        }
    }
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.work_steps()
        ),
        (records, spool, work - 1)
    );
    assert!(matches!(
        failed.write_truetype_font(closures, failed_at.unwrap()),
        Err(T::Budget(E::Work))
    ));
    assert!(failed.record_charge() >= records);
    assert!(failed.spool_charge() >= spool);
    assert_eq!(failed.work_steps(), work - 1);
}
