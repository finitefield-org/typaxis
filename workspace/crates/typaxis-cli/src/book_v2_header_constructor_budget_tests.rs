use super::*;
use typaxis_pagination::book_v2::{
    prepare_book_v2_table_body_search_counted,
    prepare_book_v2_table_body_search_with_headers_counted, BookV2TableHeaderCatalog,
};

pub(super) fn catalog(
    catalog: &BookV2TableHeaderCatalog<'_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    let mut records = 0;
    let mut work = 0;
    let prior = catalog.record_charge();
    let plain = prepare_book_v2_table_body_search_counted(
        catalog.base(),
        limits,
        1_000_000_000,
        prior,
        &mut records,
        &mut work,
    )
    .unwrap();
    let initial_work = work;
    let full = prepare_book_v2_table_body_search_with_headers_counted(
        catalog,
        limits,
        1_000_000_000,
        prior,
        &mut records,
        &mut work,
    )
    .unwrap();
    assert_eq!((records, work), (full.record_charge(), full.work_steps()));
    assert!(work > initial_work);
    let full_work = work;
    let full_records = records;
    let mut caps = vec![
        0,
        initial_work - 1,
        initial_work,
        initial_work + 1,
        full_work - 1,
    ];
    caps.retain(|cap| *cap < full_work);
    caps.sort_unstable();
    caps.dedup();
    let mut last = (prior, 0);
    for cap in caps {
        assert!(prepare_book_v2_table_body_search_with_headers_counted(
            catalog,
            limits,
            cap,
            prior,
            &mut records,
            &mut work,
        )
        .is_err());
        assert!(records >= last.0 && records <= full_records);
        assert!(work >= last.1 && work <= cap);
        if cap >= initial_work {
            assert!(records >= plain.record_charge());
            assert!(work >= initial_work);
        }
        last = (records, work);
    }
    let retained = records;
    assert!(prepare_book_v2_table_body_search_with_headers_counted(
        catalog,
        limits,
        full_work - 1,
        retained,
        &mut records,
        &mut work,
    )
    .is_err());
    assert!(records > retained && work > 0);
    let exact = prepare_book_v2_table_body_search_with_headers_counted(
        catalog,
        limits,
        full_work,
        prior,
        &mut records,
        &mut work,
    )
    .unwrap();
    assert_eq!((records, work), (full_records, full_work));
    assert_eq!(exact.body_table_count(), full.body_table_count());
    let maximum = limits.base().get().max_fragments;
    let exact_prior = maximum - (full_records - prior);
    assert!(prepare_book_v2_table_body_search_with_headers_counted(
        catalog,
        limits,
        full_work,
        exact_prior,
        &mut records,
        &mut work,
    )
    .is_ok());
    assert_eq!(records, maximum);
    assert!(prepare_book_v2_table_body_search_with_headers_counted(
        catalog,
        limits,
        full_work,
        exact_prior + 1,
        &mut records,
        &mut work,
    )
    .is_err());
    assert!(records >= exact_prior + 1 && records <= maximum);
}

pub(super) fn discovery(
    seed: &typaxis_layout::book_v2::BookV2BodyLineVariantSeed<'_>,
    limits: &M4EffectiveResourceLimits,
) {
    use crate::book_v2_resources::converged_pdf::header_catalog_driver::{
        with_discovered_header_catalog, with_header_catalog, HeaderCatalogBudget,
    };
    use crate::book_v2_resources::BookV2ConvergenceError;
    let maximum = 1_000_000_000;
    // Discovery reserves its request owner before preparing the empty catalog.
    let mut prefix = HeaderCatalogBudget {
        records: 1,
        ..Default::default()
    };
    let failures = with_header_catalog(
        seed,
        &[],
        limits,
        maximum,
        &mut prefix,
        |catalog, budget| {
            let mut records = 0;
            let mut work = 0;
            let full = prepare_book_v2_table_body_search_with_headers_counted(
                catalog,
                limits,
                maximum,
                budget.records,
                &mut records,
                &mut work,
            )
            .unwrap();
            let mut failures = Vec::new();
            for cap in [0, full.work_steps() / 2, full.work_steps() - 1] {
                let cause = prepare_book_v2_table_body_search_with_headers_counted(
                    catalog,
                    limits,
                    cap,
                    budget.records,
                    &mut records,
                    &mut work,
                )
                .err()
                .unwrap();
                failures.push((cap, records, work, cause));
            }
            Ok(failures)
        },
    )
    .unwrap();
    for (cap, records, work, cause) in failures {
        let maximum = prefix.work + cap;
        let mut budget = HeaderCatalogBudget::default();
        let error = with_discovered_header_catalog(
            seed,
            limits,
            maximum,
            &mut budget,
            |_, _| -> Result<(), BookV2ConvergenceError> {
                panic!("failed discovery constructor reached caller")
            },
        )
        .unwrap_err();
        let BookV2ConvergenceError::Stage { stage, source } = error else {
            panic!("{error:?}")
        };
        assert_eq!(stage, "header discovery search");
        assert_eq!(
            *source
                .downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>()
                .unwrap(),
            cause
        );
        assert_eq!(budget.work, prefix.work + work);
        assert_eq!(budget.records, records);
        assert_eq!(budget.line_passes, prefix.line_passes);
        assert_eq!(budget.page_passes, 1);
        let retained = budget;
        assert!(
            with_discovered_header_catalog(seed, limits, maximum, &mut budget, |_, _| Ok(()))
                .is_err()
        );
        assert!(budget.work >= retained.work && budget.work <= maximum);
        assert!(budget.records >= retained.records);
        assert!(budget.page_passes >= retained.page_passes);
    }
}
