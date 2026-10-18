//! Private from-parent controls on authentic source-context host inputs.
//! These compare actual PDF bytes and accepted prefixes; no Page-label loop is claimed.
use super::super::super::{
    marked_driver, navigation_driver, paints_driver, pdf_driver, relations_driver,
    with_budgeted_book_v2_column_display,
};
use super::*;
use typaxis_resources::book_v2::BookV2FontCommandSession;

fn quartet(value: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [value.records, value.spool, value.output, value.work]
}

#[derive(Debug, PartialEq, Eq)]
struct LowerFacts {
    fingerprints: Vec<[u8; 32]>,
    pdf: Vec<u8>,
    content: Vec<Vec<u8>>,
    constructors: [[u64; 4]; 14],
    caller_before: [u64; 4],
    previous: BookV2PdfConvergenceObservation,
    stages: [BookV2PdfConvergenceObservation; 5],
}

fn facts(
    roots: &pdf_driver::BookV2ColumnPdfBundle<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    observed: BookV2PdfConvergenceObservation,
) -> LowerFacts {
    let navigation = roots.navigation_bundle();
    let marked = navigation.marked();
    let content = marked.content();
    assert!(std::ptr::eq(roots.display(), marked.paints().display()));
    assert!(std::ptr::eq(
        navigation.relations(),
        navigation.relation_bundle().relations()
    ));
    let page_content = (0..roots.display().source().source().geometry().pages().len())
        .map(|page| content.page_bytes(page).unwrap().to_vec())
        .collect();
    LowerFacts {
        fingerprints: vec![
            roots.display().fingerprint(),
            marked.paints().text().fingerprint(),
            marked.paints().images().fingerprint(),
            content.source().fingerprint(),
            content.fingerprint(),
            marked.structure().fingerprint(),
            navigation.relations().fingerprint(),
            navigation.navigation().fingerprint(),
            roots.pdf().fingerprint(),
        ],
        pdf: roots.pdf().bytes().to_vec(),
        content: page_content,
        constructors: roots.constructor_charges(),
        caller_before: roots.caller_before(),
        previous: roots.previous_observation(),
        stages: [
            navigation.encoded_prefix(),
            navigation.marked_prefix(),
            navigation.relations_prefix(),
            roots.navigation_prefix(),
            observed,
        ],
    }
}

#[derive(Clone, Copy)]
enum Callback {
    Inspect,
    Repeat,
    RepeatThenErr,
}

// The five-argument private callbacks share the caller-owned observation rather
// than capturing it while it is already mutably borrowed by the parent helper.
macro_rules! from_paints {
    ($paints:expr, $encoded:expr, $previous:expr, $caps:expr, $maximum:expr,
     $progress:expr, $phase:expr, $callback:expr, $repeated:expr) => {{
        marked_driver::with_column_marked_from_paints(
            $paints,
            $caps,
            $maximum[0],
            $encoded,
            $progress,
            $phase,
            |marked, _, observed, phase, progress| {
                relations_driver::with_column_relations_from_marked(
                    marked,
                    $caps,
                    $maximum[1],
                    observed,
                    progress,
                    phase,
                    |relations, _, observed, phase, progress| {
                        navigation_driver::with_column_navigation_from_relations(
                            relations,
                            $caps,
                            $maximum[2],
                            observed,
                            progress,
                            phase,
                            |navigation, _, observed, phase, progress| {
                                pdf_driver::with_column_pdf_from_navigation(
                                    navigation,
                                    $caps,
                                    $maximum[3],
                                    observed,
                                    $previous,
                                    progress,
                                    phase,
                                    |pdf, continuation, observed, _, _| {
                                        let result = facts(pdf, observed);
                                        match $callback {
                                            Callback::Inspect => Ok(result),
                                            Callback::Repeat | Callback::RepeatThenErr => {
                                                let _ = continuation.build().map_err(|e| {
                                                    stage("lower continuation control", e)
                                                })?;
                                                *$repeated =
                                                    Some(BookV2PdfConvergenceObservation {
                                                        records: continuation.record_charge(),
                                                        spool: continuation.spool_charge(),
                                                        output: continuation.output_charge(),
                                                        work: continuation.work_steps(),
                                                        ..observed
                                                    });
                                                if matches!($callback, Callback::RepeatThenErr) {
                                                    Err(E::Limit("lower callback sentinel"))
                                                } else {
                                                    Ok(result)
                                                }
                                            }
                                        }
                                    },
                                )?
                            },
                        )?
                    },
                )?
            },
        )?
    }};
}

fn drive_roots(
    prepared: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    budget: &mut BookV2ColumnPageBudget,
    maximum: [u64; 4],
    callback: Callback,
    repeated: &mut Option<BookV2PdfConvergenceObservation>,
) -> Result<LowerFacts, E> {
    let previous = budget.observation();
    let mut progress = previous;
    let outcome = paints_driver::with_column_paint_phase(
        prepared,
        caps,
        MODE,
        budget,
        4,
        |paints, _, encoded, phase| {
            from_paints!(
                paints,
                encoded,
                previous,
                caps,
                maximum,
                &mut progress,
                phase,
                callback,
                repeated
            )
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(progress.records);
    budget.inner.observation.spool = budget.inner.observation.spool.max(progress.spool);
    budget.inner.observation.output = progress.output;
    budget.inner.observation.work = budget.inner.observation.work.max(progress.work);
    outcome?
}

fn drive_session(
    prepared: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    budget: &mut BookV2ColumnPageBudget,
    session: &mut BookV2FontCommandSession,
) -> Result<LowerFacts, E> {
    let previous = budget.observation();
    let mut progress = previous;
    let mut phase = paints_driver::BookV2ColumnOutputPhase::default();
    let mut repeated = None;
    let outcome = with_budgeted_book_v2_column_display(
        prepared,
        caps,
        MODE,
        budget,
        |display, _, observed| {
            progress = BookV2PdfConvergenceObservation {
                records: progress.records.max(observed.records),
                spool: progress.spool.max(observed.spool),
                output: progress.output.max(observed.output),
                work: progress.work.max(observed.work),
                ..observed
            };
            paints_driver::with_column_paints_from_session(
                display,
                session,
                caps,
                MAXIMUM,
                4,
                quartet(previous),
                &mut progress,
                &mut phase,
                |paints, _, encoded, phase, progress| {
                    from_paints!(
                        paints,
                        encoded,
                        previous,
                        caps,
                        [MAXIMUM; 4],
                        progress,
                        phase,
                        Callback::Inspect,
                        &mut repeated
                    )
                },
            )
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(progress.records);
    budget.inner.observation.spool = budget.inner.observation.spool.max(progress.spool);
    budget.inner.observation.output = progress.output;
    budget.inner.observation.work = budget.inner.observation.work.max(progress.work);
    outcome??
}

#[test]
fn book_v2_column_lower_roots_match_public_pdf_artifacts_and_all_prefixes() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let mut default_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let expected = pdf_driver::with_budgeted_book_v2_column_pdf(
        &prepared,
        &caps,
        MODE,
        &mut default_budget,
        4,
        |roots, _, observed| facts(roots, observed),
    )
    .unwrap();
    let mut roots_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let actual = drive_roots(
        &prepared,
        &caps,
        &mut roots_budget,
        [MAXIMUM; 4],
        Callback::Inspect,
        &mut None,
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(roots_budget.observation(), default_budget.observation());
    let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
    let mut session_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let session_actual =
        drive_session(&prepared, &caps, &mut session_budget, &mut session).unwrap();
    assert_eq!(session_actual, expected);
    assert_eq!(session_budget.observation(), default_budget.observation());
    assert!(session.record_charge() > 0);
    assert!(session.work_steps() > 0);
}

#[test]
fn book_v2_column_lower_roots_nested_constructor_errors_keep_accepted_prefixes() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let expected = drive_roots(
        &prepared,
        &caps,
        &mut BookV2ColumnPageBudget::new(&caps, MAXIMUM),
        [MAXIMUM; 4],
        Callback::Inspect,
        &mut None,
    )
    .unwrap();
    for (index, stage) in [
        "column marked scope constructor",
        "column structure relation constructor",
        "column navigation constructor",
        "column PDF assembly constructor",
    ]
    .into_iter()
    .enumerate()
    {
        let before = expected.stages[index];
        let mut maxima = [MAXIMUM; 4];
        // A direct helper may receive an already admitted upstream prefix above
        // its local work cap; retain that prefix and the accepted +1 record.
        maxima[index] = before.work - 1;
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let error = drive_roots(
            &prepared,
            &caps,
            &mut budget,
            maxima,
            Callback::Inspect,
            &mut None,
        )
        .unwrap_err();
        assert!(
            matches!(error, E::Stage { stage: actual, .. } if actual == stage),
            "{stage}: {error:?}"
        );
        let after = budget.observation();
        assert_eq!(after.records, before.records + 1, "{stage}");
        assert_eq!(after.spool, before.spool, "{stage}");
        assert_eq!(after.output, before.output, "{stage}");
        assert_eq!(after.work, before.work, "{stage}");
    }
}

#[test]
fn book_v2_column_lower_roots_nested_build_and_callback_errors_keep_signed_output() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let expected = drive_roots(
        &prepared,
        &caps,
        &mut BookV2ColumnPageBudget::new(&caps, MAXIMUM),
        [MAXIMUM; 4],
        Callback::Inspect,
        &mut None,
    )
    .unwrap();
    assert_ne!(expected.stages[4].output, expected.stages[3].output);
    let mut maxima = [MAXIMUM; 4];
    maxima[3] = expected.stages[4].work - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let error = drive_roots(
        &prepared,
        &caps,
        &mut budget,
        maxima,
        Callback::Inspect,
        &mut None,
    )
    .unwrap_err();
    assert!(
        matches!(
            error,
            E::Stage {
                stage: "column PDF assembly",
                ..
            }
        ),
        "{error:?}"
    );
    let after = budget.observation();
    assert_eq!(after.records, expected.stages[4].records);
    assert_eq!(after.spool, expected.stages[4].spool);
    assert_eq!(after.output, expected.stages[4].output);
    assert_eq!(after.work, maxima[3]);
    for callback in [Callback::Repeat, Callback::RepeatThenErr] {
        let mut repeated = None;
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let result = drive_roots(
            &prepared,
            &caps,
            &mut budget,
            [MAXIMUM; 4],
            callback,
            &mut repeated,
        );
        if matches!(callback, Callback::RepeatThenErr) {
            assert!(matches!(result, Err(E::Limit("lower callback sentinel"))));
        } else {
            assert_eq!(result.unwrap(), expected);
        }
        let actual = repeated.unwrap();
        assert!(actual.records > expected.stages[4].records);
        assert!(actual.spool > expected.stages[4].spool);
        assert!(actual.output > expected.stages[4].output);
        assert!(actual.work > expected.stages[4].work);
        assert_eq!(budget.observation(), actual);
    }
}
