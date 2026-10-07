use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[path = "book_v2_font_closure_tests.rs"]
mod font_closures;
use typaxis_resources::book_v2::{
    BookV2FontSelectionBuilder as Builder, BookV2FontSelectionError as E,
};

pub(super) fn check(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let maximum = display.work_steps().checked_add(10_000_000).unwrap();
    let mut expected = BTreeMap::<_, BTreeSet<_>>::new();
    let mut positions = Vec::new();
    for paint in 0..display.paints().len() {
        for slot in 0..display.font_slot_count(paint).unwrap() {
            if let Some(usage) = display.font_use(paint, slot).unwrap() {
                positions.push((paint, slot));
                let set = expected
                    .entry(usage.instance().font_instance_id())
                    .or_default();
                for glyph in 0..usage.glyphs().len() {
                    set.insert(usage.glyphs().get(glyph).unwrap());
                }
            }
        }
    }
    let run = |prior, spool, work, work_before| -> Result<_, E> {
        let mut builder = Builder::new(display, limits, work, prior, spool, work_before)?;
        let selected = builder.build()?;
        assert!(std::ptr::eq(selected.display(), display));
        assert_eq!(selected.record_charge(), builder.record_charge());
        assert_eq!(selected.spool_charge(), builder.spool_charge());
        assert_eq!(selected.work_steps(), builder.work_steps());
        assert_eq!(selected.uses().len(), positions.len());
        for (actual, &(paint, slot)) in selected.uses().iter().zip(&positions) {
            assert_eq!(actual.paint_index(), paint);
            assert_eq!(actual.usage().slot(), slot);
            let source = display.font_use(paint, slot).unwrap().unwrap();
            assert_eq!(actual.usage().source(), source.source());
            assert_eq!(actual.usage().text(), source.text());
            assert_eq!(actual.usage().size(), source.size());
        }
        assert_eq!(selected.fonts().len(), expected.len());
        for (i, (font, (id, gids))) in selected.fonts().iter().zip(&expected).enumerate() {
            assert_eq!(font.instance().font_instance_id(), *id);
            assert!(std::ptr::eq(font.instance().ledger(), display.admitted()));
            assert_eq!(
                selected.glyphs(i).unwrap().collect::<Vec<_>>(),
                gids.iter().copied().collect::<Vec<_>>()
            );
        }
        assert!(selected.glyphs(selected.fonts().len()).is_none());
        let repeated = builder.build()?;
        assert_eq!(repeated.fingerprint(), selected.fingerprint());
        assert!(repeated.record_charge() > selected.record_charge());
        assert!(repeated.work_steps() > selected.work_steps());
        assert!(repeated.spool_charge() >= selected.spool_charge());
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            selected.fingerprint(),
        ))
    };
    let (records, spool, work, fp) = run(0, 0, maximum, 0).unwrap();
    let mut closure_builder = Builder::new(display, limits, maximum, 0, 0, 0).unwrap();
    let selection = closure_builder.build().unwrap();
    font_closures::check(&selection, limits);
    let mut single = Builder::new(display, limits, work, 0, 0, 0).unwrap();
    let before = single.record_charge();
    single.build().unwrap();
    let reserved = single.record_charge();
    let reserved_spool = single.spool_charge();
    let mut lower = display.work_steps();
    let mut upper = single.work_steps() - 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut probe = Builder::new(display, limits, middle, 0, 0, 0).unwrap();
        assert_eq!(probe.build().err().unwrap(), E::Work);
        if probe.record_charge() == reserved {
            upper = middle;
        } else {
            assert_eq!(probe.record_charge(), before);
            lower = middle + 1;
        }
    }
    let mut early = Builder::new(display, limits, upper, 0, 0, 0).unwrap();
    assert_eq!(early.build().err().unwrap(), E::Work);
    assert_eq!(
        (
            early.record_charge(),
            early.spool_charge(),
            early.work_steps()
        ),
        (reserved, reserved_spool, upper)
    );
    assert_eq!(early.build().err().unwrap(), E::Work);
    assert!(early.record_charge() >= reserved);
    assert!(early.spool_charge() >= reserved_spool);
    assert_eq!(early.work_steps(), upper);
    let prior = limits.base().get().max_fragments - (records - display.record_charge());
    let prior_spool =
        limits.base().get().max_spool_bytes - (spool - display.source().spool_charge());
    assert_eq!(
        run(prior, prior_spool, work, 0).unwrap(),
        (
            limits.base().get().max_fragments,
            limits.base().get().max_spool_bytes,
            work,
            fp
        )
    );
    assert_eq!(
        run(prior + 1, prior_spool, work, 0).unwrap_err(),
        E::Records
    );
    assert_eq!(run(prior, prior_spool + 1, work, 0).unwrap_err(), E::Spool);
    assert_eq!(run(0, 0, work - 1, 0).unwrap_err(), E::Work);
    assert_eq!(
        run(0, 0, work + 17, display.work_steps() + 17).unwrap(),
        (records, spool, work + 17, fp)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    failed.build().unwrap();
    assert_eq!(failed.build().err().unwrap(), E::Work);
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.work_steps()
        ),
        (records, spool, work - 1)
    );
    assert_eq!(failed.build().err().unwrap(), E::Work);
    assert!(failed.record_charge() >= records);
    assert!(failed.spool_charge() >= spool);
    assert_eq!(failed.work_steps(), work - 1);
}
