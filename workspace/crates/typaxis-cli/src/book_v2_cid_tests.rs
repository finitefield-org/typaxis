use super::*;
#[path = "book_v2_font_stream_tests.rs"]
mod streams;
use std::collections::{BTreeMap, BTreeSet};
use typaxis_display_list::book_v2::BookV2FontUseText;
use typaxis_resources::book_v2::{BookV2CidError as C, BookV2FontPrograms};
fn original(text: BookV2FontUseText<'_>) -> String {
    match text {
        BookV2FontUseText::Text(s) => s.to_owned(),
        BookV2FontUseText::Scalar(c) => c.to_string(),
    }
}
pub(super) fn check(
    programs: &BookV2FontPrograms<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let selection = programs.source().selection();
    let display = selection.display();
    const MAX_WORK: u64 = 1_000_000_000;
    let mut stream_builder = Builder::new(display, limits, MAX_WORK, 0, 0, 0).unwrap();
    let stream_cids = stream_builder.plan_cids(programs).unwrap();
    streams::check(&stream_cids, limits);
    // Independent set oracle: an ambiguous set can never become unique when
    // another occurrence repeats an earlier scalar.
    let mut claims = BTreeMap::<_, BTreeSet<char>>::new();
    for source in selection.uses() {
        let usage = source.usage();
        let chars = original(usage.text()).chars().collect::<Vec<_>>();
        if usage.glyphs().len() == 1 && chars.len() == 1 {
            claims
                .entry((
                    usage.instance().font_instance_id(),
                    usage.glyphs().get(0).unwrap(),
                ))
                .or_default()
                .insert(chars[0]);
        }
    }
    let unicode = |font, gid| {
        claims
            .get(&(font, gid))
            .filter(|s| s.len() == 1)
            .and_then(|s| s.first())
            .copied()
    };
    let run = |prior, spool, work, prior_work| -> Result<_, C> {
        let mut builder = Builder::new(display, limits, work, prior, spool, prior_work)?;
        let stats = builder.cff_program_statistics();
        let first = builder.plan_cids(programs)?;
        assert_eq!(builder.cff_program_statistics(), stats);
        assert!(std::ptr::eq(first.source(), programs));
        assert_eq!(first.fonts().len(), programs.fonts().len());
        assert_eq!(first.uses().len(), selection.uses().len());
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
        for (index, (font, source)) in first.fonts().iter().zip(programs.fonts()).enumerate() {
            assert!(std::ptr::eq(font.source(), source));
            let glyphs = selection.glyphs(index).unwrap().collect::<Vec<_>>();
            let bindings = first.bindings(index).unwrap();
            assert_eq!(bindings.len(), glyphs.len());
            let instance = source.source().source().instance();
            assert_eq!(font.font_instance_id(), instance.font_instance_id());
            let units = u64::from(instance.font().metadata().units_per_em);
            for (ordinal, (binding, gid)) in bindings.iter().zip(glyphs).enumerate() {
                assert_eq!(binding.original_gid(), gid);
                assert_ne!(gid.get(), 0);
                assert_eq!(binding.cid().get() as usize, ordinal + 1);
                assert_eq!(binding.subset_gid(), source.subset_gid(gid).unwrap());
                let advance = u64::from(source.advance(gid).unwrap());
                assert_eq!(
                    binding.width_1000() as u64,
                    (advance * 1000 + units / 2) / units
                );
                assert_eq!(binding.unicode(), unicode(font.font_instance_id(), gid));
                if source.kind() == K::Cff1V2 {
                    assert_eq!(binding.cid().get(), binding.subset_gid().get());
                }
            }
        }
        assert!(first.bindings(first.fonts().len()).is_none());
        for (index, (usage, source)) in first.uses().iter().zip(selection.uses()).enumerate() {
            assert!(std::ptr::eq(usage.source(), source));
            assert_eq!(
                first.usage_index(source.paint_index(), source.usage().slot()),
                Some(index)
            );
            let font = &first.fonts()[usage.font_index()];
            assert_eq!(
                font.font_instance_id(),
                source.usage().instance().font_instance_id()
            );
            let cids = first.cids(index).unwrap();
            assert_eq!(cids.len(), source.usage().glyphs().len());
            let bindings = first.bindings(usage.font_index()).unwrap();
            let mut extracted = String::new();
            for (i, cid) in cids.iter().enumerate() {
                let gid = source.usage().glyphs().get(i).unwrap();
                let binding = &bindings[cid.get() as usize - 1];
                assert_eq!(binding.original_gid(), gid);
                if let Some(c) = unicode(font.font_instance_id(), gid) {
                    extracted.push(c);
                }
            }
            let expected = original(source.usage().text());
            assert_eq!(usage.requires_actual_text(), extracted != expected);
            if extracted != expected {
                assert_eq!(usage.actual_text().unwrap(), source.usage().text());
                extracted = original(usage.actual_text().unwrap());
            } else {
                assert!(usage.actual_text().is_none());
            }
            assert_eq!(extracted, expected);
        }
        assert!(first.cids(first.uses().len()).is_none());
        assert!(first.usage_index(usize::MAX, usize::MAX).is_none());
        for paint in 0..display.paints().len() {
            for slot in 0..display.font_slot_count(paint).unwrap() {
                assert_eq!(
                    first.usage_index(paint, slot).is_some(),
                    display.font_use(paint, slot).unwrap().is_some()
                );
            }
        }
        let again = builder.plan_cids(programs)?;
        assert_eq!(again.fingerprint(), first.fingerprint());
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            first.fingerprint(),
            first.record_charge(),
            first.spool_charge(),
            first.work_steps(),
        ))
    };
    let (records, spool, work, fp, first_records, first_spool, first_work) =
        run(0, 0, MAX_WORK, 0).unwrap();
    let baseline = (display.record_charge() + 1).max(programs.record_charge());
    let before_spool = display.source().spool_charge().max(programs.spool_charge());
    let prior = limits.base().get().max_fragments - (records - baseline) - 1;
    let prior_spool = limits.base().get().max_spool_bytes - (spool - before_spool);
    let exact = run(prior, prior_spool, work, 0).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2, exact.3),
        (
            limits.base().get().max_fragments,
            limits.base().get().max_spool_bytes,
            work,
            fp
        )
    );
    assert!(matches!(
        run(prior + 1, prior_spool, work, 0),
        Err(C::Budget(E::Records))
    ));
    assert!(matches!(
        run(prior, prior_spool + 1, work, 0),
        Err(C::Budget(E::Spool))
    ));
    assert!(matches!(run(0, 0, work - 1, 0), Err(C::Budget(E::Work))));
    let prior_work = run(0, 0, work + 17, programs.work_steps() + 17).unwrap();
    assert_eq!(
        (prior_work.0, prior_work.1, prior_work.2, prior_work.3),
        (records, spool, work + 17, fp)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    failed.plan_cids(programs).unwrap();
    assert!(matches!(
        failed.plan_cids(programs),
        Err(C::Budget(E::Work))
    ));
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.work_steps()
        ),
        (records, spool, work - 1)
    );
    assert!(matches!(
        failed.plan_cids(programs),
        Err(C::Budget(E::Work))
    ));
    assert!(failed.record_charge() >= records && failed.spool_charge() >= spool);
    let mut lower = programs.work_steps();
    let mut upper = first_work - 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut probe = Builder::new(display, limits, middle, 0, 0, 0).unwrap();
        assert!(matches!(probe.plan_cids(programs), Err(C::Budget(E::Work))));
        if probe.record_charge() == first_records {
            upper = middle;
        } else {
            assert_eq!(probe.record_charge(), baseline);
            lower = middle + 1;
        }
    }
    let mut early = Builder::new(display, limits, lower, 0, 0, 0).unwrap();
    assert!(matches!(early.plan_cids(programs), Err(C::Budget(E::Work))));
    assert_eq!(
        (
            early.record_charge(),
            early.spool_charge(),
            early.work_steps()
        ),
        (first_records, first_spool, lower)
    );
    assert!(matches!(early.plan_cids(programs), Err(C::Budget(E::Work))));
    assert!(early.record_charge() >= first_records && early.spool_charge() >= first_spool);
    assert_eq!(early.work_steps(), lower);
    let mut other_builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
        display.source(),
        display.admitted(),
        limits,
        MAX_WORK,
        0,
        0,
    )
    .unwrap();
    let other = other_builder.build_body().unwrap();
    assert_eq!(other.fingerprint(), display.fingerprint());
    let mut foreign = Builder::new(&other, limits, MAX_WORK, 0, 0, 0).unwrap();
    assert!(matches!(
        foreign.plan_cids(programs),
        Err(C::Budget(E::Identity))
    ));
}
