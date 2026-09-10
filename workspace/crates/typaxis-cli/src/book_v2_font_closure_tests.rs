use super::*;
#[path = "book_v2_cff_program_tests.rs"]
mod cff_programs;
#[path = "book_v2_cff_subset_tests.rs"]
mod cff_subsets;
#[path = "book_v2_font_program_tests.rs"]
mod font_programs;
#[path = "book_v2_truetype_subset_tests.rs"]
mod truetype_subsets;
use typaxis_resources::book_v2::{
    BookV2FontClosureError as C, BookV2FontClosureKind as K, BookV2FontSelection,
};

pub(super) fn check(
    selection: &BookV2FontSelection<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let maximum = selection.work_steps().checked_add(10_000_000).unwrap();
    let display = selection.display();
    let mut other_builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
        display.source(),
        display.admitted(),
        limits,
        maximum,
        0,
        0,
    )
    .unwrap();
    let other = other_builder.build_body().unwrap();
    assert_eq!(other.fingerprint(), display.fingerprint());
    let mut foreign = Builder::new(&other, limits, maximum, 0, 0, 0).unwrap();
    assert!(matches!(
        foreign.prepare_font_closures(selection),
        Err(C::Budget(E::Identity))
    ));
    let mut writer_builder = Builder::new(display, limits, maximum, 0, 0, 0).unwrap();
    let prepared = writer_builder.prepare_font_closures(selection).unwrap();
    truetype_subsets::check(&prepared, limits);
    cff_programs::check(&prepared, limits);
    cff_subsets::check(&prepared, limits);
    font_programs::check(&prepared, limits);
    assert!(matches!(
        foreign.write_font_programs(&prepared),
        Err(typaxis_resources::book_v2::BookV2FontProgramsError::Budget(
            E::Identity
        ))
    ));
    assert!(matches!(
        foreign.write_cff_font(&prepared, 0),
        Err(typaxis_resources::book_v2::BookV2CffSubsetError::Budget(
            E::Identity
        ))
    ));
    assert!(matches!(
        foreign.prepare_cff_programs(&prepared),
        Err(typaxis_resources::book_v2::BookV2CffProgramError::Budget(
            E::Identity
        ))
    ));
    assert!(matches!(
        foreign.write_truetype_font(&prepared, 0),
        Err(typaxis_resources::book_v2::BookV2TrueTypeSubsetError::Budget(E::Identity))
    ));
    let run = |prior, spool, work, work_before| -> Result<_, C> {
        let mut builder = Builder::new(display, limits, work, prior, spool, work_before)?;
        let first = builder.prepare_font_closures(selection)?;
        assert!(std::ptr::eq(first.selection(), selection));
        assert_eq!(first.record_charge(), builder.record_charge());
        assert_eq!(first.spool_charge(), builder.spool_charge());
        assert_eq!(first.work_steps(), builder.work_steps());
        assert_eq!(first.fonts().len(), selection.fonts().len());
        for (index, (closed, source)) in first.fonts().iter().zip(selection.fonts()).enumerate() {
            assert!(std::ptr::eq(closed.source(), source));
            let expected = selection.glyphs(index).unwrap().collect::<BTreeSet<_>>();
            let actual = closed.glyphs().collect::<Vec<_>>();
            assert_eq!(actual[0].get(), 0);
            assert_eq!(closed.glyphs().len(), actual.len());
            assert!(actual.windows(2).all(|p| p[0] < p[1]));
            assert!(expected.iter().all(|gid| actual.binary_search(gid).is_ok()));
            let font = source.instance().font();
            assert!(actual
                .iter()
                .all(|gid| u32::from(gid.get()) < font.metadata().glyph_count));
            for (dense, original) in actual.iter().enumerate() {
                assert_eq!(closed.subset_gid(*original).unwrap().get() as usize, dense);
            }
            assert!(closed
                .subset_gid(typaxis_font::OriginalGlyphId::new(u16::MAX))
                .is_none());
            match font {
                typaxis_resources::AdmittedProductionFontV3::TrueType(_) => {
                    assert_eq!(closed.kind(), K::TrueType)
                }
                typaxis_resources::AdmittedProductionFontV3::Cff1V2(_) => {
                    assert_eq!(closed.kind(), K::Cff1V2);
                    assert_eq!(actual.len(), expected.len() + 1);
                    assert_eq!(actual[1..], expected.iter().copied().collect::<Vec<_>>());
                }
            }
        }
        let again = builder.prepare_font_closures(selection)?;
        assert_eq!(again.fingerprint(), first.fingerprint());
        assert_eq!(
            again
                .fonts()
                .iter()
                .map(|f| f.fingerprint())
                .collect::<Vec<_>>(),
            first
                .fonts()
                .iter()
                .map(|f| f.fingerprint())
                .collect::<Vec<_>>()
        );
        assert!(again.record_charge() > first.record_charge());
        assert!(again.work_steps() > first.work_steps());
        assert!(again.spool_charge() >= first.spool_charge());
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            first.fingerprint(),
        ))
    };
    let (records, spool, work, fp) = run(0, 0, maximum, 0).unwrap();
    // A new builder takes one owner record before inheriting selection lower
    // bounds; use observed baseline to preserve that exact accounting.
    let baseline = (display.record_charge() + 1).max(selection.record_charge());
    let before_spool = display
        .source()
        .spool_charge()
        .max(selection.spool_charge());
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
        Err(C::Budget(E::Records))
    ));
    assert!(matches!(
        run(prior, prior_spool + 1, work, 0),
        Err(C::Budget(E::Spool))
    ));
    assert!(matches!(run(0, 0, work - 1, 0), Err(C::Budget(E::Work))));
    assert_eq!(
        run(0, 0, work + 17, selection.work_steps() + 17).unwrap(),
        (records, spool, work + 17, fp)
    );
    let mut failed = Builder::new(display, limits, work - 1, 0, 0, 0).unwrap();
    failed.prepare_font_closures(selection).unwrap();
    assert!(matches!(
        failed.prepare_font_closures(selection),
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
        failed.prepare_font_closures(selection),
        Err(C::Budget(E::Work))
    ));
    assert!(failed.record_charge() >= records);
    assert!(failed.spool_charge() >= spool);
    assert_eq!(failed.work_steps(), work - 1);
}
