use super::*;
#[path = "book_v2_cid_tests.rs"]
mod cids;
use typaxis_resources::book_v2::{BookV2FontClosures, BookV2FontProgramsError as P};
pub(super) fn check(
    closures: &BookV2FontClosures<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let display = closures.selection().display();
    const MAX_WORK: u64 = 1_000_000_000;
    let mut prepared_builder = Builder::new(display, limits, MAX_WORK, 0, 0, 0).unwrap();
    let prepared = prepared_builder.write_font_programs(closures).unwrap();
    cids::check(&prepared, limits);
    let run = |prior, spool, work, before_work| -> Result<_, P> {
        let mut builder = Builder::new(display, limits, work, prior, spool, before_work)?;
        let first = builder.write_font_programs(closures)?;
        assert!(std::ptr::eq(first.source(), closures));
        assert_eq!(first.fonts().len(), closures.fonts().len());
        assert_eq!(
            (
                first.record_charge(),
                first.spool_charge(),
                first.work_steps()
            ),
            (
                builder.record_charge(),
                builder.spool_charge(),
                builder.work_steps()
            )
        );
        for (font, closed) in first.fonts().iter().zip(closures.fonts()) {
            assert!(std::ptr::eq(font.source(), closed));
            assert_eq!(font.kind(), closed.kind());
            assert_eq!(font.sha256(), typaxis_core::sha256(font.bytes()));
            assert!(font.postscript_name().ends_with("+Typaxis"));
            match font.kind() {
                K::TrueType => assert_eq!(&font.bytes()[..4], &[0, 1, 0, 0]),
                K::Cff1V2 => assert!(font.bytes().starts_with(b"OTTO")),
            }
            for gid in closed.glyphs() {
                assert_eq!(font.subset_gid(gid), closed.subset_gid(gid));
                assert!(font.advance(gid).is_some());
            }
            assert!(font
                .advance(typaxis_font::OriginalGlyphId::new(u16::MAX))
                .is_none());
            assert!(font.metrics().ascent_1000 > 0);
        }
        let stats = builder.cff_program_statistics();
        let again = builder.write_font_programs(closures)?;
        assert_eq!(again.fingerprint(), first.fingerprint());
        assert_eq!(builder.cff_program_statistics(), stats);
        for (a, b) in first.fonts().iter().zip(again.fonts()) {
            assert_eq!(a.bytes(), b.bytes());
            assert_eq!(a.metrics(), b.metrics());
            assert_eq!(a.fingerprint(), b.fingerprint());
        }
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            first.fingerprint(),
        ))
    };
    let (records, spool, work, fp) = run(0, 0, MAX_WORK, 0).unwrap();
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
            fp
        )
    );
    assert!(matches!(
        run(prior + 1, prior_spool, work, 0),
        Err(P::Budget(E::Records))
    ));
    assert!(matches!(
        run(prior, prior_spool + 1, work, 0),
        Err(P::Budget(E::Spool))
    ));
    assert!(matches!(run(0, 0, work - 1, 0), Err(P::Budget(E::Work))));
    assert_eq!(
        run(0, 0, work + 17, closures.work_steps() + 17).unwrap(),
        (records, spool, work + 17, fp)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    failed.write_font_programs(closures).unwrap();
    let stats = failed.cff_program_statistics();
    assert!(matches!(
        failed.write_font_programs(closures),
        Err(P::Budget(E::Work))
    ));
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.work_steps()
        ),
        (records, spool, work - 1)
    );
    assert_eq!(failed.cff_program_statistics(), stats);
    assert!(matches!(
        failed.write_font_programs(closures),
        Err(P::Budget(E::Work))
    ));
    assert!(failed.record_charge() >= records && failed.spool_charge() >= spool);
    assert_eq!(failed.work_steps(), work - 1);
    assert_eq!(failed.cff_program_statistics(), stats);
}
