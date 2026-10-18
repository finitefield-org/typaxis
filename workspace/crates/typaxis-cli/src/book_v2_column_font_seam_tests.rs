//! Genuine TrueType adapter controls on the shared source-context host harness.
//! These are private paint-stage controls, without a CFF-positive/PDF claim.
use super::super::super::{paints_driver, with_budgeted_book_v2_column_display};
use super::*;
use paints_driver::{BookV2ColumnOutputPhase, BookV2ColumnPaintBundle};
use typaxis_resources::book_v2::BookV2FontCommandSession;

fn quartet(v: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [v.records, v.spool, v.output, v.work]
}
#[derive(Debug, PartialEq, Eq)]
struct PaintFacts {
    fingerprints: Vec<[u8; 32]>,
    payloads: Vec<Vec<u8>>,
    actual_text: Vec<Option<Vec<u8>>>,
    constructors: [[u64; 4]; 8],
    caller_before: [u64; 4],
    charges: [u64; 4],
}
fn facts(
    roots: &BookV2ColumnPaintBundle<
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
) -> PaintFacts {
    let text = roots.text();
    let objects = text.source();
    let streams = objects.source();
    let cids = streams.source();
    let programs = cids.source();
    let closures = programs.source();
    let selection = closures.selection();
    let images = roots.images();
    let image_objects = images.source();
    let vectors = image_objects.source();
    let rasters = vectors.source();
    let image_selection = rasters.source();
    assert!(std::ptr::eq(roots.display(), selection.display()));
    assert!(std::ptr::eq(roots.display(), image_selection.display()));
    let mut payloads = vec![
        objects.bytes().to_vec(),
        image_objects.bytes().to_vec(),
        images.bytes().to_vec(),
    ];
    for program in programs.fonts() {
        payloads.push(program.bytes().to_vec());
    }
    for font in 0..streams.fonts().len() {
        payloads.push(streams.to_unicode(font).unwrap().to_vec());
        payloads.push(streams.auxiliary(font).unwrap().to_vec());
    }
    let mut actual_text = Vec::new();
    for usage in 0..cids.uses().len() {
        actual_text.push(streams.actual_text(usage).map(<[u8]>::to_vec));
    }
    for command in 0..text.commands().len() {
        payloads.push(text.command_bytes(command).unwrap().to_vec());
    }
    PaintFacts {
        fingerprints: vec![
            roots.display().fingerprint(),
            selection.fingerprint(),
            closures.fingerprint(),
            programs.fingerprint(),
            cids.fingerprint(),
            streams.fingerprint(),
            objects.fingerprint(),
            text.fingerprint(),
            image_selection.fingerprint(),
            rasters.fingerprint(),
            vectors.fingerprint(),
            image_objects.fingerprint(),
            images.fingerprint(),
        ],
        payloads,
        actual_text,
        constructors: roots.constructor_charges(),
        caller_before: roots.caller_before(),
        charges: quartet(observed),
    }
}

fn drive_session(
    prepared: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    budget: &mut BookV2ColumnPageBudget,
    session: &mut BookV2FontCommandSession,
    repeat_then_err: bool,
) -> Result<(Result<PaintFacts, &'static str>, Option<[u64; 4]>), E> {
    let before = budget.observation();
    let mut progress = before;
    let mut repeated = None;
    let mut phase = BookV2ColumnOutputPhase::default();
    let outcome = with_budgeted_book_v2_column_display(
        prepared,
        caps,
        MODE,
        budget,
        |display, _, observed| {
            progress.records = progress.records.max(observed.records);
            progress.spool = progress.spool.max(observed.spool);
            progress.output = progress.output.max(observed.output);
            progress.work = progress.work.max(observed.work);
            progress.line_passes = observed.line_passes;
            progress.page_passes = observed.page_passes;
            paints_driver::with_column_paints_from_session(
                display,
                session,
                caps,
                MAXIMUM,
                4,
                quartet(before),
                &mut progress,
                &mut phase,
                |roots, builder, observed, _, _| {
                    let result = facts(roots, observed);
                    if repeat_then_err {
                        let _ = builder.build().unwrap();
                        repeated = Some([
                            builder.record_charge(),
                            builder.spool_charge(),
                            builder.output_charge(),
                            builder.work_steps(),
                        ]);
                        Err("font seam callback sentinel")
                    } else {
                        Ok(result)
                    }
                },
            )
        },
    );
    // The production seam returned genuine typed stage observations. This
    // harness commits that result exactly as the future private caller will.
    budget.inner.observation.records = budget.inner.observation.records.max(progress.records);
    budget.inner.observation.spool = budget.inner.observation.spool.max(progress.spool);
    budget.inner.observation.output = progress.output;
    budget.inner.observation.work = budget.inner.observation.work.max(progress.work);
    outcome?.map(|result| (result, repeated))
}

#[test]
fn book_v2_column_font_seam_command_adapter_matches_default_artifacts_and_counters() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let mut raw_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let raw = super::super::super::paints_driver::with_budgeted_book_v2_column_paints(
        &prepared,
        &caps,
        MODE,
        &mut raw_budget,
        4,
        |roots, _, observed| facts(roots, observed),
    )
    .unwrap();
    let mut command_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
    let command = drive_session(&prepared, &caps, &mut command_budget, &mut session, false)
        .unwrap()
        .0
        .unwrap();
    assert_eq!(command, raw);
    assert_eq!(command_budget.observation(), raw_budget.observation());
    assert!(!command.payloads.iter().all(Vec::is_empty));
    let stats = session.cff_program_statistics();
    assert_eq!(
        (
            stats.operations_used,
            stats.outline_segments_used,
            stats.cached_glyphs
        ),
        (0, 0, 0)
    );
    eprintln!(
        "Column font-seam raw/command equality: {:?}; constructors={:?}; payload_sha={:?}",
        command.charges,
        command.constructors,
        command
            .payloads
            .iter()
            .map(|p| typaxis_core::sha256(p))
            .collect::<Vec<_>>()
    );
}

#[test]
fn book_v2_column_font_seam_nested_callback_error_and_retry_keep_full_prefix() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
    let first = drive_session(&prepared, &caps, &mut budget, &mut session, false)
        .unwrap()
        .0
        .unwrap();
    let first_total = budget.observation();
    let failed = drive_session(&prepared, &caps, &mut budget, &mut session, true).unwrap();
    assert_eq!(failed.0, Err("font seam callback sentinel"));
    let expected = failed.1.unwrap();
    assert_eq!(quartet(budget.observation()), expected);
    assert!(expected
        .into_iter()
        .zip(quartet(first_total))
        .all(|(a, b)| a > b));
    let floor = budget.observation();
    let retry = drive_session(&prepared, &caps, &mut budget, &mut session, false)
        .unwrap()
        .0
        .unwrap();
    assert_eq!(retry.caller_before, quartet(floor));
    assert_eq!(retry.fingerprints, first.fingerprints);
    assert_eq!(retry.payloads, first.payloads);
    assert_eq!(retry.actual_text, first.actual_text);
    assert!(retry.constructors[0][0] > floor.records && retry.constructors[0][3] > floor.work);
    assert!(
        session.record_charge() >= retry.constructors[4][0]
            && session.work_steps() >= retry.constructors[4][3]
    );
    eprintln!(
        "Column font-seam callback/retry prefixes: first={:?}; callback={expected:?}; retry={:?}",
        quartet(first_total),
        quartet(budget.observation())
    );
}

fn stage_name(error: &E) -> &str {
    match error {
        E::Stage { stage, .. } => stage,
        other => panic!("expected stage failure, got {other:?}"),
    }
}

#[test]
fn book_v2_column_font_seam_constructor_exact_one_short_and_failed_retry_keep_history() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_budgeted_book_v2_column_display(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        |display, _, observed| {
            for axis in [0, 1, 3] {
                for short in [false, true] {
                    let mut progress = observed;
                    let maximum = if axis == 3 {
                        observed.work - u64::from(short)
                    } else {
                        MAXIMUM
                    };
                    match axis {
                        0 => progress.records = caps.base().get().max_fragments - u64::from(!short),
                        1 => progress.spool = caps.base().get().max_spool_bytes + u64::from(short),
                        3 => (),
                        _ => unreachable!(),
                    }
                    let prior = progress;
                    let mut session = BookV2FontCommandSession::new(&caps, maximum);
                    let mut phase = BookV2ColumnOutputPhase::default();
                    let failed = paints_driver::with_column_paints_from_session(
                        display,
                        &mut session,
                        &caps,
                        maximum,
                        4,
                        quartet(prior),
                        &mut progress,
                        &mut phase,
                        |_, _, _, _, _| {
                            panic!("bounded constructor/first operation unexpectedly delivered")
                        },
                    );
                    let error = failed.unwrap_err();
                    assert_eq!(
                        stage_name(&error),
                        if short {
                            "column paint font constructor"
                        } else {
                            "column paint font selection"
                        }
                    );
                    assert_eq!(
                        progress.records,
                        prior.records + u64::from(axis != 0 || !short)
                    );
                    assert_eq!(progress.spool, prior.spool);
                    assert_eq!(progress.output, prior.output);
                    assert!(progress.work >= prior.work);
                    assert_eq!(
                        [
                            session.record_charge(),
                            session.spool_charge(),
                            session.work_steps()
                        ],
                        [progress.records, progress.spool, progress.work]
                    );
                    let accepted = progress;
                    let mut phase = BookV2ColumnOutputPhase::default();
                    let _ = paints_driver::with_column_paints_from_session(
                        display,
                        &mut session,
                        &caps,
                        maximum,
                        4,
                        quartet(accepted),
                        &mut progress,
                        &mut phase,
                        |_, _, _, _, _| panic!("bounded retry unexpectedly delivered"),
                    );
                    assert!(
                        progress.records >= accepted.records
                            && progress.spool >= accepted.spool
                            && progress.work >= accepted.work
                    );
                }
            }
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_font_seam_rejects_foreign_admission_and_limits_without_resetting_history() {
    let root = Root::new();
    let other_root = Root::new();
    let caps = caps();
    let source = data(false);
    let prepared = input(&root, &source, &caps);
    let foreign = input(&other_root, &source, &caps);
    assert_eq!(
        prepared.body().styled().body().canonical_jcs_sha256(),
        foreign.body().styled().body().canonical_jcs_sha256()
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
    drive_session(&prepared, &caps, &mut budget, &mut session, false)
        .unwrap()
        .0
        .unwrap();
    let floor = budget.observation();
    let changed = changed(&caps, |c| c.max_fragments += 1);
    for (input, limits) in [(&foreign, &caps), (&prepared, &changed)] {
        let mut source_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        with_budgeted_book_v2_column_display(
            input,
            &caps,
            MODE,
            &mut source_budget,
            |display, _, _| {
                let mut progress = floor;
                let mut phase = BookV2ColumnOutputPhase::default();
                let error = paints_driver::with_column_paints_from_session(
                    display,
                    &mut session,
                    limits,
                    MAXIMUM,
                    4,
                    quartet(floor),
                    &mut progress,
                    &mut phase,
                    |_, _, _, _, _| panic!("foreign command source delivered"),
                )
                .unwrap_err();
                assert_eq!(stage_name(&error), "column paint font constructor");
                assert!(format!("{error:?}").contains("Identity"));
                assert_eq!(progress, floor);
                assert_eq!(
                    [
                        session.record_charge(),
                        session.spool_charge(),
                        session.work_steps()
                    ],
                    [floor.records, floor.spool, floor.work]
                );
            },
        )
        .unwrap();
    }
    let retry = drive_session(&prepared, &caps, &mut budget, &mut session, false)
        .unwrap()
        .0
        .unwrap();
    assert!(retry.charges[0] > floor.records && retry.charges[3] > floor.work);
}
