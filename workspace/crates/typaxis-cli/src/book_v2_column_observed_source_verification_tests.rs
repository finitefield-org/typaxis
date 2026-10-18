//! Independent P0 controls use genuine source, shaping, lines and pagination.
//! The host fixture and Stage1 oracle remain unchanged. Requested source bytes
//! and bounded line/search work are observed here; allocator capacity, native
//! math and running-region replay are outside this prototype.
use super::super::super as driver;
use super::*;
use typaxis_layout::book_v2::{
    prepare_book_v2_inline_items_with_native_context_counted,
    prepare_book_v2_inline_items_with_observed_source_counted, with_budgeted_book_v2_column_lines,
    with_observed_book_v2_column_lines, BookV2BodyLineBudget, BookV2ObservedBodyLineBudget,
    BookV2ObservedBodyReshapeError, BookV2ObservedInlineError,
};
use typaxis_shaping::book_v2::{
    shape_book_v2_authored_text, shape_book_v2_authored_text_with_observed_source_counted,
    with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as A,
    BookV2ObservedShapeError, BookV2ObservedSourceAccounting, BookV2ObservedVerificationError as V,
};
use typaxis_syntax::book_v2::{
    BookV2ObservedSourceFlowError as S, BookV2ObservedSourceFlowObservation as SourceObservation,
};

const PRIOR: [u64; 3] = [7, 11, 13];

fn counters(o: SourceObservation) -> [u64; 3] {
    [o.record_charge(), o.spool_charge(), o.work_steps()]
}

fn oracle(
    flow: &PreparedBookV2TextFlow<'_>,
    body: &StyledBookV2Body,
    navigation: &PreparedBookV2Navigation<'_>,
    prior: [u64; 3],
    maximum_records: u64,
    maximum_work: u64,
) -> (Result<(), S>, [u64; 3]) {
    let mut observation = SourceObservation::default();
    let result = flow.verify_for_observed_counted(
        body,
        navigation,
        prior[0],
        prior[1],
        prior[2],
        maximum_records,
        maximum_work,
        &mut observation,
    );
    (result, counters(observation))
}

fn loan<'cells, R>(
    cells: &'cells mut [u64; 3],
    limits: &'cells ValidatedResourceLimits,
    maximum_records: u64,
    maximum_work: u64,
    inspect: impl for<'id> FnOnce(&mut BookV2ObservedSourceAccounting<'id, 'cells>) -> R,
) -> R {
    let [r, s, w] = cells;
    with_book_v2_observed_source_accounting(r, s, w, limits, maximum_records, maximum_work, inspect)
}

fn seeded() -> BookV2PdfConvergenceObservation {
    BookV2PdfConvergenceObservation {
        records: PRIOR[0],
        spool: PRIOR[1],
        work: PRIOR[2],
        output: 17,
        candidates: 2,
        balance_candidates: 3,
        width_passes: 4,
        width_refinements: 5,
        line_passes: 6,
        page_passes: 7,
    }
}

fn rsw(o: BookV2PdfConvergenceObservation) -> [u64; 3] {
    [o.records, o.spool, o.work]
}

fn page_facts(pages: &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>) -> Value {
    json!(pages
        .sequence()
        .pages()
        .iter()
        .map(|page| json!({
            "index":page.page_index(),
            "name":page.named_page(),
            "forced":page.forced_break().map(|o|o.get()),
            "columns":page.candidate().columns().iter().map(|column| json!({
                "x":column.bounds().x().raw(),"y":column.bounds().y().raw(),
                "width":column.bounds().width().get().raw(),
                "height":column.bounds().height().get().raw(),
                "used":column.used_height().raw(),
                "parts":column.parts().iter().map(|part|part.items()).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        }))
        .collect::<Vec<_>>())
}

#[test]
fn book_v2_column_observed_verification_actual_stage1_exact_short_all_axes() {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let maximum_records = limits.base().get().max_fragments;
    let (result, full) = oracle(&flow, body, &navigation, PRIOR, maximum_records, MAXIMUM);
    result.unwrap();
    assert!(full.iter().zip(PRIOR).all(|(a, b)| *a > b));
    for axis in 0..3 {
        for short in [false, true] {
            let mut start = PRIOR;
            let max_r = if axis == 0 {
                full[0] - u64::from(short)
            } else {
                maximum_records
            };
            // This is allowance arithmetic on the authentic S domain, not an
            // assertion about allocator capacity or a historical allocation.
            if axis == 1 {
                start[1] =
                    limits.base().get().max_spool_bytes - (full[1] - PRIOR[1]) + u64::from(short);
            }
            let max_w = if axis == 2 {
                full[2] - u64::from(short)
            } else {
                MAXIMUM
            };
            let (expected, expected_cells) = oracle(&flow, body, &navigation, start, max_r, max_w);
            let mut cells = start;
            let actual = loan(&mut cells, limits.base(), max_r, max_w, |accounting| {
                let result = accounting.verify_source(&flow, body, &navigation);
                assert_eq!(accounting.counters(), expected_cells);
                result
            });
            assert_eq!(actual, expected.map_err(V::Source));
            assert_eq!(cells, expected_cells, "axis={axis}, short={short}");
            assert_eq!(actual.is_err(), short);
            if short {
                assert!(matches!(actual, Err(V::Source(S::Records { .. }))) == (axis == 0));
                assert!(matches!(actual, Err(V::Source(S::Spool { .. }))) == (axis == 1));
                assert!(matches!(actual, Err(V::Source(S::Work { .. }))) == (axis == 2));
            }
        }
    }
    eprintln!(
        "P0 direct full source before={PRIOR:?} after={full:?}, intrinsic={}",
        flow.source_record_charge()
    );
}

#[test]
fn book_v2_column_observed_verification_shape_inline_match_real_outputs_and_intrinsic_union() {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let epoch = bindings.epoch();
    let old_shape =
        shape_book_v2_authored_text(&policy, &flow, prepared.resources(), &limits, epoch, None)
            .unwrap();
    let mut inline_records = 0;
    let old_inline = prepare_book_v2_inline_items_with_native_context_counted(
        &flow,
        &old_shape,
        prepared.resources(),
        &bindings,
        &limits,
        MODE,
        None,
        &mut inline_records,
    )
    .unwrap();
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            accounting.with_zero_intrinsic_records(|accounting, window| {
                assert_eq!(window.paid_records(), 0);
                let (expected, mut expected_cells) = oracle(
                    &flow,
                    body,
                    &navigation,
                    PRIOR,
                    limits.base().get().max_fragments,
                    MAXIMUM,
                );
                expected.unwrap();
                let mut shape_records = u64::MAX;
                let shape = shape_book_v2_authored_text_with_observed_source_counted(
                    &policy,
                    &flow,
                    prepared.resources(),
                    &limits,
                    epoch,
                    None,
                    accounting,
                    window,
                    &mut shape_records,
                )
                .unwrap();
                assert_eq!(shape.fingerprint(), old_shape.fingerprint());
                assert_eq!(shape.output_records(), old_shape.output_records());
                assert_eq!(shape_records, shape.output_records());
                expected_cells[0] += shape_records + prepared.resources().fonts().len() as u64;
                let after_shape = accounting.counters();
                assert_eq!(after_shape[0], expected_cells[0]);
                assert!(after_shape[1] > expected_cells[1] && after_shape[2] > expected_cells[2]);
                // Source verification remains an exact oracle from the actual
                // current cells, including the separately metered shaper scratch.
                expected_cells = after_shape;
                assert_eq!(window.paid_records(), shape_records);
                let (expected, mut expected_cells) = oracle(
                    &flow,
                    body,
                    &navigation,
                    expected_cells,
                    limits.base().get().max_fragments,
                    MAXIMUM,
                );
                expected.unwrap();
                let mut actual_inline_records = u64::MAX;
                let inline = prepare_book_v2_inline_items_with_observed_source_counted(
                    &flow,
                    &shape,
                    prepared.resources(),
                    &bindings,
                    &limits,
                    MODE,
                    accounting,
                    window,
                    &mut actual_inline_records,
                )
                .unwrap();
                assert_eq!(inline.fingerprint(), old_inline.fingerprint());
                assert_eq!(actual_inline_records, inline_records);
                assert_eq!(inline.paragraphs().len(), old_inline.paragraphs().len());
                assert!(inline.native_math().is_none());
                assert!(std::ptr::eq(inline.source_flow(), &flow));
                assert!(std::ptr::eq(inline.shaped(), &shape));
                expected_cells[0] += inline_records.saturating_sub(shape_records);
                assert_eq!(accounting.counters(), expected_cells);
                assert_eq!(window.paid_records(), shape_records.max(inline_records));
                window.seal();
            });
        },
    );
    assert!(cells.iter().zip(PRIOR).all(|(a, b)| *a > b));
    eprintln!(
        "P0 shape+inline before={PRIOR:?} after={cells:?}, shapeR={}, inlineR={inline_records}",
        old_shape.output_records()
    );
}

#[test]
fn book_v2_column_observed_verification_nested_source_columns_then_full_verify_sees_current_cells()
{
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, MAXIMUM, 0, 0).unwrap();
    assert!(flow.tables().is_empty());
    let mut legacy_budget =
        BookV2BodyLineBudget::new(MAXIMUM, limits.base().get().max_line_reshape_passes);
    let (old_lines, old_pages) = with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        prepared.resources(),
        &bindings,
        &limits,
        MODE,
        None,
        &mut legacy_budget,
        &plan,
        None,
        |stable| {
            let columns = prepare_book_v2_column_flow_counted(
                &stable,
                None,
                &limits,
                stable.retained_record_charge(),
                &mut 0,
            )
            .unwrap();
            assert_eq!(columns.table_count(), 0);
            let measured =
                prepare_book_v2_column_table_measurements_counted(columns, &limits, &mut 0)
                    .unwrap();
            assert!(measured.tables().is_empty());
            let mut search = prepare_book_v2_column_page_search_counted(
                &measured,
                &limits,
                MAXIMUM,
                measured.record_charge(),
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let pages = search
                .select_repeated_pages_counted(limits.base().get().max_layout_passes, &mut 0)
                .unwrap();
            (stable.lines().fingerprint(), page_facts(&pages))
        },
    )
    .unwrap();
    let prior = seeded();
    let mut total = prior;
    driver::with_observed_column_progress(
        &mut total,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting, rest| {
            let mut local = BookV2ObservedBodyLineBudget::new(
                MAXIMUM,
                limits.base().get().max_line_reshape_passes,
            );
            let result = with_observed_book_v2_column_lines(
                &policy,
                &flow,
                prepared.resources(),
                &bindings,
                &limits,
                MODE,
                &mut local,
                accounting,
                &plan,
                None,
                |stable, accounting| {
                    assert_eq!(stable.lines().fingerprint(), old_lines);
                    assert!(std::ptr::eq(stable.column_plan(), &plan));
                    assert_eq!(
                        stable.passes().len(),
                        legacy_budget.reshape_passes() as usize
                    );
                    let before = accounting.counters();
                    let mut view = driver::ColumnProgressView {
                        accounting: &mut *accounting,
                        rest: &mut *rest,
                    };
                    let callback_after = driver::with_p0_observed_column_pagination(
                        &stable,
                        &limits,
                        &mut view,
                        |pages, search, feedback, observed| {
                            assert_eq!(page_facts(pages), old_pages);
                            assert!(pages.sequence().pages().len() > 0 && pages.passes() >= 2);
                            assert!(feedback.matches_source_flow(&flow));
                            assert!(std::ptr::eq(feedback.column_plan(), &plan));
                            assert!(std::ptr::eq(search.source_lines(), stable.lines()));
                            assert_eq!(observed.records, search.record_charge());
                            assert_eq!(observed.spool, search.terminal_spool_charge());
                            assert_eq!(observed.work, before[2] + search.work_steps());
                            assert_eq!(observed.candidates, prior.candidates + 1);
                            assert!(observed.page_passes > prior.page_passes);
                            assert_eq!(observed.output, prior.output);
                            let initial_records = search.record_charge();
                            let initial_work = search.work_steps();
                            // verify_sequence is an uncharged immutable receipt
                            // check. Real feedback reconstruction allocates its
                            // own carrier and spends charged search R and W.
                            search.verify_sequence(pages.sequence()).unwrap();
                            assert_eq!(search.work_steps(), initial_work);
                            let replay = search.paragraph_frame_feedback(pages.sequence()).unwrap();
                            assert_eq!(
                                replay.assignment_fingerprint(),
                                feedback.assignment_fingerprint()
                            );
                            assert!(search.record_charge() > initial_records);
                            assert!(search.work_steps() > initial_work);
                            (
                                [
                                    search.record_charge(),
                                    search.terminal_spool_charge(),
                                    before[2] + search.work_steps(),
                                ],
                                observed,
                            )
                        },
                    )
                    .unwrap();
                    assert_eq!(view.accounting.counters(), callback_after.0);
                    assert_eq!(view.snapshot().output, prior.output);
                    callback_after.1
                },
            );
            *rest.line_passes += local.reshape_passes();
            result.unwrap();
            // All nested page/search/flow loans ended. The same genuine source is
            // recollected now, and must inherit the exact current accepted triple.
            let before = accounting.counters();
            let (expected, after) = oracle(
                &flow,
                body,
                &navigation,
                before,
                limits.base().get().max_fragments,
                MAXIMUM,
            );
            expected.unwrap();
            accounting.verify_source(&flow, body, &navigation).unwrap();
            assert_eq!(accounting.counters(), after);
            assert!(after.iter().zip(before).all(|(a, b)| *a > b));
            assert_eq!(*rest.candidates, prior.candidates + 1);
            assert_eq!(*rest.width_passes, prior.width_passes);
            assert_eq!(*rest.width_refinements, prior.width_refinements);
            assert_eq!(*rest.balance_candidates, prior.balance_candidates);
            assert_eq!(*rest.output, prior.output);
            eprintln!("P0 genuine nested SourceColumns before-verify={before:?} after-verify={after:?}; local-candidates={} line-passes={}",local.candidate_steps(),local.reshape_passes());
        },
    );
    assert!(rsw(total).iter().zip(PRIOR).all(|(a, b)| *a > b));
    assert!(total.line_passes > prior.line_passes && total.page_passes > prior.page_passes);
    assert_eq!(total.output, prior.output);
}

#[test]
fn book_v2_column_observed_verification_current_work_precedes_measure_and_is_not_double_added() {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, MAXIMUM, 0, 0).unwrap();
    let (verified, once) = oracle(
        &flow,
        body,
        &navigation,
        PRIOR,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    verified.unwrap();
    let run = |maximum_work| {
        let mut cells = PRIOR;
        let mut local =
            BookV2ObservedBodyLineBudget::new(MAXIMUM, limits.base().get().max_line_reshape_passes);
        let mut called = false;
        let result = loan(
            &mut cells,
            limits.base(),
            limits.base().get().max_fragments,
            maximum_work,
            |accounting| {
                with_observed_book_v2_column_lines(
                    &policy,
                    &flow,
                    prepared.resources(),
                    &bindings,
                    &limits,
                    MODE,
                    &mut local,
                    accounting,
                    &plan,
                    None,
                    |stable, _| {
                        called = true;
                        assert!(stable.passes().last().unwrap().is_stable());
                        stable.lines().fingerprint()
                    },
                )
            },
        );
        (result, cells, local, called)
    };
    let (full, after, local, called) = run(MAXIMUM);
    let fingerprint = full.unwrap();
    assert!(called);
    let reconstructions = 2 * (u64::from(local.reshape_passes()) + 1);
    assert!(local.reshape_passes() > 0 && local.candidate_steps() > 0);
    assert_eq!(
        after[0],
        PRIOR[0]
            + reconstructions * (once[0] - PRIOR[0])
            + local.record_charge()
            + (u64::from(local.reshape_passes()) + 1) * prepared.resources().fonts().len() as u64
    );
    // The independent source/candidate oracle is a strict lower bound now
    // that every observed shape also pays its bidi/Harfrust storage and work.
    assert!(after[1] > PRIOR[1] + reconstructions * (once[1] - PRIOR[1]));
    assert!(after[2] > PRIOR[2] + reconstructions * (once[2] - PRIOR[2]) + local.candidate_steps());
    let (exact, exact_cells, exact_local, called) = run(after[2]);
    assert!(called);
    assert_eq!(exact.unwrap(), fingerprint);
    assert_eq!(exact_cells, after);
    assert_eq!(exact_local.candidate_steps(), local.candidate_steps());
    let (short, short_cells, short_local, called) = run(after[2] - 1);
    assert!(!called && short.is_err());
    assert!(matches!(
        short,
        Err(BookV2ObservedBodyReshapeError::Layout(_))
    ));
    assert!(short_local.reshape_passes() > 0);
    assert_eq!(short_cells[2], after[2] - 1);
    assert_eq!(short_local.candidate_steps() + 1, local.candidate_steps());
    eprintln!("P0 actual current-work full={after:?}, one-short={short_cells:?}, source-recollections={reconstructions}, measureW={}",local.candidate_steps());
}

#[test]
fn book_v2_column_observed_verification_limits_domain_identity_and_lower_record_floor() {
    let root = Root::new();
    let other_root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let other = input(&other_root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let foreign_navigation = prepare_book_v2_navigation(body).unwrap();
    let other_navigation = prepare_book_v2_navigation(other.body().styled()).unwrap();
    for change in [-1i64, 1] {
        let mut base = limits.base().get().clone();
        base.max_spool_bytes = if change < 0 {
            base.max_spool_bytes - 1
        } else {
            base.max_spool_bytes + 1
        };
        let changed = ValidatedResourceLimits::new(base).unwrap();
        let mut cells = PRIOR;
        let result = loan(&mut cells, &changed, u64::MAX, MAXIMUM, |accounting| {
            accounting.verify_source(&flow, body, &navigation)
        });
        assert_eq!(result, Err(V::Accounting(A::LimitsMismatch)));
        assert_eq!(cells, PRIOR);
    }
    let mut base = limits.base().get().clone();
    base.max_layout_passes -= 1;
    let changed = ValidatedResourceLimits::new(base).unwrap();
    let mut cells = PRIOR;
    assert_eq!(
        loan(&mut cells, &changed, u64::MAX, MAXIMUM, |a| a
            .verify_source(&flow, body, &navigation)),
        Err(V::Accounting(A::LimitsMismatch))
    );
    assert_eq!(cells, PRIOR);
    for (foreign_body, foreign_nav) in [
        (body, &foreign_navigation),
        (other.body().styled(), &other_navigation),
    ] {
        let (expected, after) = oracle(
            &flow,
            foreign_body,
            foreign_nav,
            PRIOR,
            limits.base().get().max_fragments,
            MAXIMUM,
        );
        assert!(expected.is_err());
        let mut cells = PRIOR;
        let actual = loan(&mut cells, limits.base(), u64::MAX, MAXIMUM, |a| {
            assert_eq!(a.maximum_records(), limits.base().get().max_fragments);
            a.verify_source(&flow, foreign_body, foreign_nav)
        });
        assert_eq!(actual, expected.map_err(V::Source));
        assert_eq!(cells, after);
    }
    let (ok, full) = oracle(
        &flow,
        body,
        &navigation,
        PRIOR,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    ok.unwrap();
    let max_r = full[0] - 1;
    let (expected, after) = oracle(&flow, body, &navigation, PRIOR, max_r, MAXIMUM);
    let mut cells = PRIOR;
    let actual = loan(&mut cells, limits.base(), max_r, MAXIMUM, |a| {
        a.verify_source(&flow, body, &navigation)
    });
    assert_eq!(actual, expected.map_err(V::Source));
    assert_eq!(cells, after);
    // Lower caller M is supported for source/lines; pagination explicitly
    // refuses that different domain before constructing any of its carriers.
    let mut total = seeded();
    driver::with_observed_column_progress(
        &mut total,
        limits.base(),
        max_r,
        MAXIMUM,
        |accounting, rest| {
            let view = driver::ColumnProgressView { accounting, rest };
            assert!(matches!(
                view.require_pagination_domain(&limits),
                Err(E::ObservedAccounting(A::LimitsMismatch))
            ));
            assert_eq!(view.snapshot(), seeded());
        },
    );
    assert_eq!(total, seeded());
}

#[test]
fn book_v2_column_observed_verification_error_order_preserves_actual_paid_source_prefix() {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let changed = changed(&limits, |base| base.max_layout_passes -= 1);
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            accounting.with_zero_intrinsic_records(|accounting,window| {
        let mut output=u64::MAX;
        let error=shape_book_v2_authored_text_with_observed_source_counted(&policy,&flow,prepared.resources(),&changed,[0;32],None,accounting,window,&mut output).err().unwrap();
        assert!(matches!(error,BookV2ObservedShapeError::Shape(e) if e.kind==typaxis_shaping::ProductionTextShapeErrorKind::ReceiptMismatch));
        assert_eq!(output,0);assert_eq!(accounting.counters(),PRIOR);
        let (expected,after)=oracle(&flow,body,&navigation,PRIOR,limits.base().get().max_fragments,MAXIMUM);expected.unwrap();
        let error=shape_book_v2_authored_text_with_observed_source_counted(&policy,&flow,prepared.resources(),&limits,[0;32],None,accounting,window,&mut output).err().unwrap();
        assert!(matches!(error,BookV2ObservedShapeError::Shape(e) if e.kind==typaxis_shaping::ProductionTextShapeErrorKind::ReceiptMismatch));
        assert_eq!(output,0);assert_eq!(accounting.counters(),after);
        // Unicode versions are compile-time dependencies, not a forged error
        // input. The actual wrong later epoch above exercises source-first order.
        assert_eq!(typaxis_core::REGISTERED_UNICODE_VERSION,"16.0.0");
    })
        },
    );
    let other_root = Root::new();
    let other = input(&other_root, &data(false), &limits);
    let other_policy = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    let other_bindings = bind_book_v2_vectors(&other_policy, other.resources(), &limits).unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        prepared.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            accounting.with_zero_intrinsic_records(|accounting, window| {
            let mut output = u64::MAX;
            let error = prepare_book_v2_inline_items_with_observed_source_counted(
                &flow, &shape, prepared.resources(), &other_bindings, &limits, MODE,
                accounting, window, &mut output,
            ).err().unwrap();
            assert!(matches!(error, BookV2ObservedInlineError::Inline(e)
                if e.kind == typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch));
            assert_eq!(output, 0);
            assert_eq!(accounting.counters(), PRIOR);
            assert_eq!(window.paid_records(), 0);
        });
        },
    );
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            accounting.with_zero_intrinsic_records(|accounting,window| {
        let wrong_epoch=if bindings.epoch()==[77;32]{[78;32]}else{[77;32]};
        let shape=shape_book_v2_authored_text(&policy,&flow,prepared.resources(),&limits,wrong_epoch,None).unwrap();
        let mut expected_start=PRIOR;expected_start[0]+=shape.output_records();
        let (expected,after)=oracle(&flow,body,&navigation,expected_start,limits.base().get().max_fragments,MAXIMUM);expected.unwrap();
        let mut output=u64::MAX;
        let error=prepare_book_v2_inline_items_with_observed_source_counted(&flow,&shape,prepared.resources(),&bindings,&limits,MODE,accounting,window,&mut output).err().unwrap();
        assert!(matches!(error,BookV2ObservedInlineError::Inline(e) if e.kind==typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch));
        assert_eq!(output,0);assert_eq!(accounting.counters(),after);
        assert_eq!(window.paid_records(),shape.output_records());
    })
        },
    );
}

#[test]
fn book_v2_column_observed_verification_callback_error_forget_and_retry_keep_exact_cells() {
    let root = Root::new();
    let limits = caps();
    let prepared = input(&root, &data(false), &limits);
    let body = prepared.body().styled();
    let navigation = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &navigation).unwrap();
    let mut expected = PRIOR;
    for _ in 0..2 {
        let (ok, after) = oracle(
            &flow,
            body,
            &navigation,
            expected,
            limits.base().get().max_fragments,
            MAXIMUM,
        );
        ok.unwrap();
        expected = after;
    }
    let mut cells = PRIOR;
    let mut during = None;
    let application: Result<(), &'static str> = loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            accounting.verify_source(&flow, body, &navigation).unwrap();
            accounting.verify_source(&flow, body, &navigation).unwrap();
            during = Some(accounting.counters());
            let borrowed = &mut *accounting;
            #[allow(forgetting_references)]
            std::mem::forget(borrowed);
            Err("actual callback application refusal")
        },
    );
    assert_eq!(application, Err("actual callback application refusal"));
    assert_eq!(during, Some(expected));
    assert_eq!(cells, expected);
    let (ok, after_retry) = oracle(
        &flow,
        body,
        &navigation,
        cells,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    ok.unwrap();
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| a.verify_source(&flow, body, &navigation),
    )
    .unwrap();
    assert_eq!(cells, after_retry);
    let (ok, full_next) = oracle(
        &flow,
        body,
        &navigation,
        cells,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    ok.unwrap();
    let maximum = full_next[2] - 1;
    let (expected_error, after_error) = oracle(
        &flow,
        body,
        &navigation,
        cells,
        limits.base().get().max_fragments,
        maximum,
    );
    let actual = loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        maximum,
        |a| a.verify_source(&flow, body, &navigation),
    );
    assert_eq!(actual, expected_error.map_err(V::Source));
    assert!(matches!(actual, Err(V::Source(S::Work { .. }))));
    assert_eq!(cells, after_error);
    assert!(after_error.iter().zip(after_retry).all(|(a, b)| *a > b));
    eprintln!("P0 callback actual app-error prefix={expected:?}, retry={after_retry:?}, work-error={after_error:?}");
}

#[test]
fn book_v2_column_observed_verification_sealed_window_and_atomic_debits_never_revive_credit() {
    let limits = caps();
    let owner = NodeId::new(0);
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            accounting.with_zero_intrinsic_records(|accounting, window| {
                accounting
                    .retain_intrinsic_output(window, owner, 4)
                    .unwrap();
                let paid = accounting.counters();
                assert_eq!(paid, [PRIOR[0] + 4, PRIOR[1], PRIOR[2]]);
                accounting
                    .retain_intrinsic_output(window, owner, 3)
                    .unwrap();
                assert_eq!(accounting.counters(), paid);
                window.seal();
                window.seal();
                assert_eq!(accounting.intrinsic_limit(window), Err(A::ClosedWindow));
                assert_eq!(
                    accounting.retain_intrinsic_output(window, owner, 8),
                    Err(A::ClosedWindow)
                );
                assert_eq!(
                    accounting.retain_intrinsic_output(window, owner, 0),
                    Err(A::ClosedWindow)
                );
                assert_eq!(accounting.counters(), paid);
            });
            accounting.with_zero_intrinsic_records(|accounting, window| {
                assert_eq!(window.paid_records(), 0);
                accounting
                    .retain_intrinsic_output(window, owner, 4)
                    .unwrap();
                assert_eq!(accounting.counters(), [PRIOR[0] + 8, PRIOR[1], PRIOR[2]]);
            });
        },
    );
    for axis in 0..3 {
        let mut cells = PRIOR;
        loan(
            &mut cells,
            limits.base(),
            PRIOR[0],
            PRIOR[2],
            |accounting| {
                let mut delta = [0, 0, 0];
                delta[axis] = u64::MAX;
                let error = accounting.debit(owner, delta).unwrap_err();
                assert_eq!(
                    error,
                    match axis {
                        0 => A::Records { owner },
                        1 => A::Spool { owner },
                        _ => A::Work { owner },
                    }
                );
                assert_eq!(accounting.counters(), PRIOR);
                assert_eq!(
                    accounting.debit(owner, [1, u64::MAX, u64::MAX]),
                    Err(A::Records { owner })
                );
                assert_eq!(accounting.counters(), PRIOR);
            },
        );
        assert_eq!(cells, PRIOR);
    }
}
