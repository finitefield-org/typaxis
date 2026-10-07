use super::*;
use typaxis_resources::book_v2::{BookV2CffProgramError as P, BookV2FontClosures};

pub(super) fn check(
    closures: &BookV2FontClosures<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let maximum = closures.work_steps().checked_add(10_000_000).unwrap();
    let display = closures.selection().display();
    let mut expected = BTreeSet::new();
    for closed in closures.fonts().iter().filter(|f| f.kind() == K::Cff1V2) {
        for gid in closed.glyphs() {
            expected.insert((closed.source().instance().font().content_hash(), gid));
        }
    }
    let run = |prior, spool, work, before_work| -> Result<_, P> {
        let mut builder = Builder::new(display, limits, work, prior, spool, before_work)?;
        assert_eq!(builder.cff_program_statistics().cached_glyphs, 0);
        let first = builder.prepare_cff_programs(closures)?;
        assert_eq!(first.cached_glyphs, expected.len());
        assert_eq!(first, builder.cff_program_statistics());
        if !expected.is_empty() {
            assert!(first.operations_used > 0);
        } else {
            assert_eq!((first.operations_used, first.outline_segments_used), (0, 0));
        }
        let records = builder.record_charge();
        let spool = builder.spool_charge();
        let again = builder.prepare_cff_programs(closures)?;
        assert_eq!(again, first);
        assert_eq!(builder.record_charge(), records);
        assert_eq!(builder.spool_charge(), spool);
        Ok((records, spool, builder.work_steps(), first))
    };
    let (records, spool, work, statistics) = run(0, 0, maximum, 0).unwrap();
    if expected.is_empty() {
        return;
    }
    let mut shared = Builder::new(display, limits, maximum, 0, 0, 0).unwrap();
    let mut programs = Vec::new();
    for (index, font) in closures.fonts().iter().enumerate() {
        if font.kind() == K::TrueType {
            programs.push(shared.write_truetype_font(closures, index).unwrap());
        }
    }
    let before_cff = (
        shared.record_charge(),
        shared.spool_charge(),
        shared.work_steps(),
    );
    assert_eq!(shared.prepare_cff_programs(closures).unwrap(), statistics);
    assert!(shared.record_charge() >= before_cff.0);
    assert!(shared.spool_charge() >= before_cff.1);
    assert!(shared.work_steps() > before_cff.2);
    for (index, font) in closures.fonts().iter().enumerate() {
        if font.kind() == K::TrueType {
            let again = shared.write_truetype_font(closures, index).unwrap();
            let first = programs
                .iter()
                .find(|p| std::ptr::eq(p.source(), font))
                .unwrap();
            assert_eq!(again.bytes(), first.bytes());
        }
    }
    assert_eq!(shared.prepare_cff_programs(closures).unwrap(), statistics);
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
            statistics
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
        (records, spool, work + 17, statistics)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    assert_eq!(failed.prepare_cff_programs(closures).unwrap(), statistics);
    assert!(matches!(
        failed.prepare_cff_programs(closures),
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
    assert_eq!(failed.cff_program_statistics(), statistics);
    assert!(matches!(
        failed.prepare_cff_programs(closures),
        Err(P::Budget(E::Work))
    ));
    assert_eq!(failed.cff_program_statistics(), statistics);
    // Locate the first attempted Type2 operation. Its surrounding document
    // work can fail, but the aggregate attempt and allocation charges remain;
    // no partially evaluated glyph is admitted to the cache.
    let mut lower = closures.work_steps();
    let mut upper = work - 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut probe = Builder::new(display, limits, middle, 0, 0, 0).unwrap();
        let _ = probe.prepare_cff_programs(closures);
        if probe.cff_program_statistics().operations_used > 0 {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    let mut early = Builder::new(display, limits, lower, 0, 0, 0).unwrap();
    assert!(matches!(
        early.prepare_cff_programs(closures),
        Err(P::Budget(E::Work))
    ));
    let attempted = early.cff_program_statistics();
    assert_eq!(attempted.operations_used, 1);
    assert_eq!(attempted.cached_glyphs, 0);
    assert!(early.record_charge() > baseline);
    assert!(early.spool_charge() > before_spool);
    let spent = (
        early.record_charge(),
        early.spool_charge(),
        early.work_steps(),
    );
    assert!(matches!(
        early.prepare_cff_programs(closures),
        Err(P::Budget(E::Work))
    ));
    assert_eq!(early.cff_program_statistics(), attempted);
    assert_eq!(
        (
            early.record_charge(),
            early.spool_charge(),
            early.work_steps()
        ),
        spent
    );
}
