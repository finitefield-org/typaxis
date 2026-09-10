use super::truetype_subsets::{table, u16_at};
use super::*;
use typaxis_resources::book_v2::{BookV2CffSubsetError as F, BookV2FontClosures};

pub(super) fn check(
    closures: &BookV2FontClosures<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let display = closures.selection().display();
    // This test includes whole original cmap/UVS traversal twice, whose
    // bounded work is larger than glyph-only evaluation in preceding tests.
    const MAX_WORK: u64 = 1_000_000_000;
    let run = |prior, spool, work, before_work| -> Result<_, F> {
        let mut builder = Builder::new(display, limits, work, prior, spool, before_work)?;
        assert!(matches!(
            builder.write_cff_font(closures, closures.fonts().len()),
            Err(F::InvalidFontIndex)
        ));
        let mut hashes = Vec::new();
        for (index, closed) in closures.fonts().iter().enumerate() {
            if closed.kind() == K::TrueType {
                assert!(matches!(
                    builder.write_cff_font(closures, index),
                    Err(F::WrongFontKind)
                ));
                continue;
            }
            let written = builder.write_cff_font(closures, index)?;
            assert!(std::ptr::eq(written.source(), closed));
            assert_eq!(
                (
                    written.record_charge(),
                    written.spool_charge(),
                    written.work_steps()
                ),
                (
                    builder.record_charge(),
                    builder.spool_charge(),
                    builder.work_steps()
                )
            );
            let subset = written.program();
            let bytes = subset.bytes();
            assert!(bytes.starts_with(b"OTTO"));
            assert_eq!(subset.sha256(), typaxis_core::sha256(bytes));
            assert_eq!(
                subset.closure().font_instance_id(),
                closed.source().instance().font_instance_id()
            );
            assert_eq!(
                subset.closure().source_gids(),
                closed.glyphs().collect::<Vec<_>>()
            );
            assert_eq!(
                u16_at(table(bytes, 0, b"maxp"), 4) as usize,
                closed.glyphs().len()
            );
            assert_eq!(
                u16_at(table(bytes, 0, b"hhea"), 34) as usize,
                closed.glyphs().len()
            );
            let typaxis_resources::AdmittedProductionFontV3::Cff1V2(source) =
                closed.source().instance().font()
            else {
                unreachable!()
            };
            let hmtx = table(bytes, 0, b"hmtx");
            for (dense, gid) in closed.glyphs().enumerate() {
                assert_eq!(
                    subset.original_to_subset()[&gid],
                    closed.subset_gid(gid).unwrap()
                );
                let advance = source.admission().horizontal_metric(gid.get()).unwrap().0;
                assert_eq!(subset.original_widths()[&gid], advance);
                assert_eq!(u16_at(hmtx, dense * 4), advance);
            }
            assert_eq!(u16_at(table(bytes, 0, b"head"), 18), 1000);
            assert!(subset.metrics().ascent_1000 > 0);
            assert!(subset.metrics().descent_1000 < 0);
            let names = table(bytes, 0, b"name");
            let at = (0..u16_at(names, 2) as usize)
                .map(|n| 6 + n * 12)
                .find(|at| u16_at(names, at + 6) == 6)
                .unwrap();
            let start = u16_at(names, 4) as usize + u16_at(names, at + 10) as usize;
            let length = u16_at(names, at + 8) as usize;
            let units = names[start..start + length]
                .chunks_exact(2)
                .map(|p| u16::from_be_bytes(p.try_into().unwrap()))
                .collect::<Vec<_>>();
            assert_eq!(
                String::from_utf16(&units).unwrap(),
                subset.postscript_name()
            );
            let stats = builder.cff_program_statistics();
            let again = builder.write_cff_font(closures, index)?;
            assert_eq!(again.program().bytes(), bytes);
            assert_eq!(again.program().metrics(), subset.metrics());
            assert_eq!(again.fingerprint(), written.fingerprint());
            assert_eq!(builder.cff_program_statistics(), stats);
            hashes.push(written.fingerprint());
        }
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            hashes,
        ))
    };
    let (records, spool, work, hashes) = run(0, 0, MAX_WORK, 0).unwrap();
    if hashes.is_empty() {
        return;
    }
    let baseline = (display.record_charge() + 1).max(closures.record_charge());
    let before_spool = display.source().spool_charge().max(closures.spool_charge());
    let prior = limits.base().get().max_fragments - (records - baseline) - 1;
    let prior_spool = limits.base().get().max_spool_bytes - (spool - before_spool);
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
        Err(F::Budget(E::Records))
    ));
    assert!(matches!(
        run(prior, prior_spool + 1, work, 0),
        Err(F::Budget(E::Spool))
    ));
    assert!(matches!(run(0, 0, work - 1, 0), Err(F::Budget(E::Work))));
    assert_eq!(
        run(0, 0, work + 17, closures.work_steps() + 17).unwrap(),
        (records, spool, work + 17, hashes)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    let mut failure = None;
    'fonts: for (index, closed) in closures.fonts().iter().enumerate() {
        if closed.kind() != K::Cff1V2 {
            continue;
        }
        for _ in 0..2 {
            match failed.write_cff_font(closures, index) {
                Ok(_) => (),
                Err(F::Budget(E::Work)) => {
                    failure = Some(index);
                    break 'fonts;
                }
                Err(e) => panic!("unexpected CFF subset failure: {e}"),
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
    let stats = failed.cff_program_statistics();
    assert!(matches!(
        failed.write_cff_font(closures, failure.unwrap()),
        Err(F::Budget(E::Work))
    ));
    assert_eq!(failed.cff_program_statistics(), stats);
    assert!(failed.record_charge() >= records && failed.spool_charge() >= spool);
    assert_eq!(failed.work_steps(), work - 1);
    // Both real font formats can be retained on one aggregate budget owner.
    let mut mixed = Builder::new(display, limits, MAX_WORK, 0, 0, 0).unwrap();
    let mut tt = Vec::new();
    let mut cff = Vec::new();
    for (index, closed) in closures.fonts().iter().enumerate() {
        match closed.kind() {
            K::TrueType => tt.push(mixed.write_truetype_font(closures, index).unwrap()),
            K::Cff1V2 => cff.push(mixed.write_cff_font(closures, index).unwrap()),
        }
    }
    assert_eq!(tt.len() + cff.len(), closures.fonts().len());
}
