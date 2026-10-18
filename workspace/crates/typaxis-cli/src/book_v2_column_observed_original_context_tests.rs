//! Stage2 uses the genuine Stage1 source constructor at creation. Host fixtures
//! borrow the unchanged source-context harness; they do not supply receipts.
use super::*;
use typaxis_syntax::book_v2::{
    prepare_book_v2_text_flow_observed_counted, BookV2ObservedSourceFlowError as SourceError,
    BookV2ObservedSourceFlowObservation as SourceObservation,
};

fn rsw(value: BookV2PdfConvergenceObservation) -> [u64; 3] {
    [value.records, value.spool, value.work]
}
fn observed(value: SourceObservation) -> [u64; 3] {
    [
        value.record_charge(),
        value.spool_charge(),
        value.work_steps(),
    ]
}
fn seeded() -> BookV2PdfConvergenceObservation {
    BookV2PdfConvergenceObservation {
        records: 7,
        spool: 11,
        output: 17,
        work: 13,
        candidates: 2,
        balance_candidates: 3,
        width_passes: 4,
        width_refinements: 5,
        line_passes: 6,
        page_passes: 7,
    }
}
fn same_flow(a: &PreparedBookV2TextFlow<'_>, b: &PreparedBookV2TextFlow<'_>) {
    assert!(std::ptr::eq(a.body(), b.body()));
    assert!(std::ptr::eq(a.navigation(), b.navigation()));
    assert_eq!(a.fingerprint(), b.fingerprint());
    assert_eq!(a.source_record_charge(), b.source_record_charge());
    assert_eq!(a.events(), b.events());
    assert_eq!(a.paragraphs(), b.paragraphs());
    assert_eq!(a.tables(), b.tables());
    assert_eq!(a.figures(), b.figures());
    assert_eq!(a.lists(), b.lists());
    assert_eq!(a.list_items(), b.list_items());
    assert_eq!(a.footnote_definitions(), b.footnote_definitions());
    assert_eq!(a.named_page_breaks(), b.named_page_breaks());
    assert_eq!(a.page_reference_values(), b.page_reference_values());
}

#[test]
fn book_v2_column_observed_original_source_helper_matches_genuine_stage1_without_changing_other_ledgers(
) {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(true), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let prior = seeded();
    let mut direct = SourceObservation::default();
    let expected = prepare_book_v2_text_flow_observed_counted(
        body,
        &navigation,
        prior.records,
        prior.spool,
        prior.work,
        limits.base().get().max_fragments,
        MAXIMUM,
        &mut direct,
    )
    .unwrap();
    let mut total = prior;
    let actual = prepare_original_source(
        body,
        &navigation,
        &limits,
        MAXIMUM,
        &mut total,
        OriginalSourcePreparation::Observed,
    )
    .unwrap();
    same_flow(&actual, &expected);
    assert_eq!(rsw(total), observed(direct));
    assert_eq!(
        total,
        BookV2PdfConvergenceObservation {
            records: direct.record_charge(),
            spool: direct.spool_charge(),
            work: direct.work_steps(),
            ..prior
        }
    );
    assert_eq!(
        actual.source_record_charge(),
        direct.logical_source_record_charge()
    );
    assert!(total.records > prior.records + actual.source_record_charge());
    assert!(total.spool > prior.spool && total.work > prior.work);
    eprintln!(
        "Stage2 source helper actual={:?} intrinsic={}",
        rsw(total),
        actual.source_record_charge()
    );
}

#[test]
fn book_v2_column_observed_original_context_preserves_legacy_flow_plan_and_real_ten_hundred_candidates(
) {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(true), &limits);
    let mut old = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let old_facts =
        with_original_column_context(&prepared, &limits, MODE, &mut old, |context, _| {
            Ok((
                context.flow.fingerprint(),
                context.flow.source_record_charge(),
                (
                    context.columns.measurement_body(),
                    context.columns.measurement_footnote(),
                    context.columns.minimum_body_height(),
                    context.columns.minimum_footnote_height(),
                    context.columns.requires_width_reflow(),
                    context.columns.requests_last_page_balance(),
                ),
            ))
        })
        .unwrap();
    let mut new = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    with_observed_original_column_context(&prepared, &limits, MODE, &mut new, |context, budget| {
        assert_eq!(
            (
                context.flow.fingerprint(),
                context.flow.source_record_charge(),
                (
                    context.columns.measurement_body(),
                    context.columns.measurement_footnote(),
                    context.columns.minimum_body_height(),
                    context.columns.minimum_footnote_height(),
                    context.columns.requires_width_reflow(),
                    context.columns.requests_last_page_balance(),
                )
            ),
            old_facts
        );
        assert_eq!(context.prepared_progress, budget.observation());
        assert!(budget.observation().records > old.observation().records);
        assert!(budget.observation().spool > old.observation().spool);
        assert!(budget.observation().work > old.observation().work);
        assert_eq!(budget.observation().output, old.observation().output);
        let census = context.census_counted(budget)?;
        assert_eq!(census.owners(), authored_owners(&data(true)));
        assert_eq!(census.owners().len(), 5);
        for number in [10, 100] {
            let labels = values(&census, number);
            let before = budget.observation();
            context.with_page_references(&census, &labels, budget, |candidate, budget| {
                assert!(std::ptr::eq(candidate.original, context));
                assert!(std::ptr::eq(candidate.flow.body(), context.flow.body()));
                assert!(std::ptr::eq(
                    candidate.flow.navigation(),
                    context.navigation
                ));
                assert!(std::ptr::eq(candidate.original.columns, context.columns));
                assert_eq!(
                    candidate.flow.page_reference_values(),
                    Some(labels.as_slice())
                );
                for owner in census.owners() {
                    assert_eq!(
                        candidate.flow.page_reference_text(*owner),
                        Some(number.to_string().as_str())
                    );
                }
                assert!(rsw(budget.observation())
                    .iter()
                    .zip(rsw(before))
                    .all(|(a, b)| *a > b));
                let direct = prepare_book_v2_text_flow_with_page_references(
                    context.flow.body(),
                    context.navigation,
                    &labels,
                )
                .unwrap();
                same_flow(candidate.flow, &direct);
                Ok(())
            })?;
        }
        assert!(context.flow.page_reference_values().is_none());
        Ok(())
    })
    .unwrap();
}

#[test]
fn book_v2_column_observed_original_source_exact_short_axes_match_direct_typed_prefixes_and_retry()
{
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(true), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let prior = seeded();
    let mut full = prior;
    prepare_original_source(
        body,
        &navigation,
        &limits,
        MAXIMUM,
        &mut full,
        OriginalSourcePreparation::Observed,
    )
    .unwrap();
    for axis in 0..3 {
        for short in [false, true] {
            let mut start = prior;
            let caps = if axis == 0 {
                bounded(&limits, 0, full.records - u64::from(short))
            } else {
                limits.clone()
            };
            if axis == 1 {
                start.spool = body.body().limits().get().max_spool_bytes
                    - (full.spool - prior.spool)
                    + u64::from(short);
            }
            let maximum = if axis == 2 {
                full.work - u64::from(short)
            } else {
                MAXIMUM
            };
            let mut direct = SourceObservation::default();
            let expected = prepare_book_v2_text_flow_observed_counted(
                body,
                &navigation,
                start.records,
                start.spool,
                start.work,
                caps.base().get().max_fragments,
                maximum,
                &mut direct,
            );
            let mut total = start;
            let actual = prepare_original_source(
                body,
                &navigation,
                &caps,
                maximum,
                &mut total,
                OriginalSourcePreparation::Observed,
            );
            assert_eq!(rsw(total), observed(direct));
            assert_eq!(
                total,
                BookV2PdfConvergenceObservation {
                    records: direct.record_charge(),
                    spool: direct.spool_charge(),
                    work: direct.work_steps(),
                    ..start
                }
            );
            if short {
                let expected = expected.err().unwrap();
                assert!(matches!(
                    (axis, expected),
                    (0, SourceError::Records { .. })
                        | (1, SourceError::Spool { .. })
                        | (2, SourceError::Work { .. })
                ));
                let error = actual.err().unwrap();
                assert!(matches!(&error,E::ObservedSource(source) if *source==expected));
                assert!(std::error::Error::source(&error)
                    .unwrap()
                    .downcast_ref::<SourceError>()
                    .is_some());
            } else {
                same_flow(&actual.unwrap(), &expected.unwrap());
                match axis {
                    0 => assert_eq!(total.records, caps.base().get().max_fragments),
                    1 => assert_eq!(total.spool, body.body().limits().get().max_spool_bytes),
                    2 => assert_eq!(total.work, maximum),
                    _ => unreachable!(),
                }
            }
        }
    }
    let mut retried = full;
    prepare_original_source(
        body,
        &navigation,
        &limits,
        MAXIMUM,
        &mut retried,
        OriginalSourcePreparation::Observed,
    )
    .unwrap();
    assert!(rsw(retried).iter().zip(rsw(full)).all(|(a, b)| *a > b));
    assert_eq!(retried.output, prior.output);
}

#[test]
fn book_v2_column_observed_original_source_entry_floors_precede_foreign_identity_and_keep_all_history(
) {
    let a = Root::new();
    let b = Root::new();
    let limits = caps();
    let one = input(&a, &data(false), &limits);
    let two = input(&b, &data(false), &limits);
    let foreign = prepare_book_v2_navigation(two.body().styled()).unwrap();
    for axis in 0..3 {
        let mut total = seeded();
        match axis {
            0 => total.records = limits.base().get().max_fragments + 1,
            1 => total.spool = limits.base().get().max_spool_bytes + 1,
            2 => total.work = MAXIMUM + 1,
            _ => unreachable!(),
        }
        let prior = total;
        let error = prepare_original_source(
            one.body().styled(),
            &foreign,
            &limits,
            MAXIMUM,
            &mut total,
            OriginalSourcePreparation::Observed,
        )
        .err()
        .unwrap();
        assert!(matches!(
            (axis, error),
            (0, E::ObservedSource(SourceError::Records { .. }))
                | (1, E::ObservedSource(SourceError::Spool { .. }))
                | (2, E::ObservedSource(SourceError::Work { .. }))
        ));
        assert_eq!(total, prior);
    }
    let mut direct = SourceObservation::default();
    let prior = seeded();
    let expected = prepare_book_v2_text_flow_observed_counted(
        one.body().styled(),
        &foreign,
        prior.records,
        prior.spool,
        prior.work,
        limits.base().get().max_fragments,
        MAXIMUM,
        &mut direct,
    )
    .err()
    .unwrap();
    let mut total = prior;
    let error = prepare_original_source(
        one.body().styled(),
        &foreign,
        &limits,
        MAXIMUM,
        &mut total,
        OriginalSourcePreparation::Observed,
    )
    .err()
    .unwrap();
    assert!(matches!(error, E::ObservedSource(SourceError::Source(_))));
    assert!(matches!(error, E::ObservedSource(source) if source == expected));
    assert_eq!(rsw(total), observed(direct));
    assert_eq!(
        total,
        BookV2PdfConvergenceObservation {
            records: direct.record_charge(),
            spool: direct.spool_charge(),
            work: direct.work_steps(),
            ..prior
        }
    );
    assert_eq!(total.work, prior.work + 1);
}

#[test]
fn book_v2_column_observed_original_context_callback_and_identity_errors_retain_the_authentic_prepared_prefix(
) {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let mut expected = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    with_observed_original_column_context(&prepared, &limits, MODE, &mut expected, |_, _| Ok(()))
        .unwrap();
    let mut failed = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let result = with_observed_original_column_context(
        &prepared,
        &limits,
        MODE,
        &mut failed,
        |context, budget| {
            assert_eq!(context.prepared_progress, budget.observation());
            let mut reset = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            identity(context.census_counted(&mut reset).err().unwrap());
            let mut changed_max = BookV2ColumnPageBudget::new(&limits, MAXIMUM - 1);
            changed_max.inner.observation = budget.observation();
            identity(context.census_counted(&mut changed_max).err().unwrap());
            let other = changed(&limits, |c| c.max_pages -= 1);
            let mut changed_limits = BookV2ColumnPageBudget::new(&other, MAXIMUM);
            changed_limits.inner.observation = budget.observation();
            identity(context.census_counted(&mut changed_limits).err().unwrap());
            Err::<(), E>(E::Limit("Stage2 callback sentinel"))
        },
    );
    assert!(matches!(result, Err(E::Limit("Stage2 callback sentinel"))));
    assert_eq!(failed.observation(), expected.observation());
    let before = failed.observation();
    with_observed_original_column_context(&prepared, &limits, MODE, &mut failed, |_, _| Ok(()))
        .unwrap();
    assert!(rsw(failed.observation())
        .iter()
        .zip(rsw(before))
        .all(|(a, b)| *a > b));
}

#[test]
fn book_v2_column_observed_original_outer_pdf_no_page_retry_preserves_bytes_and_all_accepted_source_prefixes(
) {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let mut legacy = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let expected = super::super::super::pdf_driver::with_budgeted_book_v2_column_pdf(
        &prepared,
        &limits,
        MODE,
        &mut legacy,
        4,
        |roots, _, _| {
            (
                roots.pdf().bytes().to_vec(),
                roots.pdf().fingerprint(),
                roots.display().fingerprint(),
            )
        },
    )
    .unwrap();
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limits, MAXIMUM);
    let mut earlier = budget.observation();
    for attempt in 0..2 {
        let mut delivered = None;
        let result = with_budgeted_converged_book_v2_column_pdf(
            &prepared,
            &limits,
            MODE,
            &mut budget,
            4,
            |roots, observed| {
                assert_eq!(
                    (
                        roots.pdf().bytes().to_vec(),
                        roots.pdf().fingerprint(),
                        roots.display().fingerprint()
                    ),
                    expected
                );
                assert!(rsw(observed).iter().zip(rsw(earlier)).all(|(a, b)| *a > b));
                delivered = Some(observed);
                Err::<BookV2PdfConvergenceObservation, &'static str>("Stage2 application sentinel")
            },
        )
        .unwrap();
        assert!(matches!(result, Err("Stage2 application sentinel")));
        let after = budget.observation();
        assert_eq!(Some(after), delivered);
        assert!(rsw(after).iter().zip(rsw(earlier)).all(|(a, b)| *a > b));
        assert!(after.line_passes > earlier.line_passes && after.page_passes > earlier.page_passes);
        earlier = after;
        eprintln!("Stage2 outer PDF application-error retry={attempt} actual={after:?}");
    }
}

#[test]
fn book_v2_column_observed_original_outer_source_work_refusal_has_typed_cause_and_no_attempt() {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limits, 0);
    let mut called = false;
    let result = with_budgeted_converged_book_v2_column_pdf(
        &prepared,
        &limits,
        MODE,
        &mut budget,
        4,
        |_, _| called = true,
    );
    assert!(!called);
    assert!(matches!(
        result,
        Err(E::ObservedSource(SourceError::Work { .. }))
    ));
    let observed = budget.observation();
    assert!(observed.records > 0);
    assert_eq!(observed.work, 0);
    assert_eq!(observed.output, 0);
    assert_eq!(
        (
            observed.candidates,
            observed.width_passes,
            observed.line_passes,
            observed.page_passes
        ),
        (0, 0, 0, 0)
    );
    let prior = observed;
    let _ = with_budgeted_converged_book_v2_column_pdf(
        &prepared,
        &limits,
        MODE,
        &mut budget,
        4,
        |_, _| (),
    );
    assert!(budget.observation().records > prior.records);
    assert_eq!(budget.observation().work, 0);
    assert_eq!(budget.observation().output, 0);
}
