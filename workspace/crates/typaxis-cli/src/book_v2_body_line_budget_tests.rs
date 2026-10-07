use super::*;
#[path = "book_v2_line_context_record_tests.rs"]
mod context_records;
#[path = "book_v2_downstream_budget_tests.rs"]
mod downstream;
#[path = "book_v2_footnote_constructor_budget_tests.rs"]
mod footnote_constructor;
#[path = "book_v2_frame_line_record_tests.rs"]
mod frame_line_records;
#[path = "book_v2_page_constructor_budget_tests.rs"]
mod page_constructor;
#[path = "book_v2_projection_body_budget_tests.rs"]
mod projection_constructor;
#[path = "book_v2_shape_inline_record_tests.rs"]
mod shape_inline_records;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, with_budgeted_book_v2_body_lines_with_source_widths as budgeted,
    with_converged_book_v2_body_lines_with_source_widths as fresh, BookV2BodyLineBudget,
    BookV2SourceWidthAssignments,
};
use typaxis_linebreak::JapaneseLineBreakMode;
const MODE: JapaneseLineBreakMode = JapaneseLineBreakMode::Normal;

pub(super) fn wide_data(text: &str) -> Value {
    let mut data = source_data(text);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 12_000_000.into();
    master["height"] = 12_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":10_000_000});
    data
}
fn input(
    root: &Root,
    text: &str,
    font: Option<&[u8]>,
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    let mut data = wide_data(text);
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        let body = body_with_source(root, data, text.as_bytes(), limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            limits,
        )
        .unwrap()
    } else {
        prepared(root, data, text.as_bytes(), limits)
    }
}
#[test]
fn book_v2_body_line_budget_retains_failed_candidates_and_started_passes() {
    check_candidates("Result Result", None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_body_line_budget_retains_original_harano_failures() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_candidates("本文", Some(&bytes));
}
fn check_candidates(text: &str, font: Option<&[u8]>) {
    let root = Root::new();
    let limits = limits();
    let input = input(&root, text, font, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let width = PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap();
    let body = Rect::new(
        Length::from_raw(500_000).unwrap(),
        Length::from_raw(500_000).unwrap(),
        width,
        width,
    );
    let run = |budget: &mut BookV2BodyLineBudget,
               assignments: Option<&BookV2SourceWidthAssignments<'_, '_>>| {
        budgeted(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            MODE,
            body,
            None,
            budget,
            None,
            assignments,
            |stable| {
                (
                    stable.lines().fingerprint(),
                    stable.candidate_steps(),
                    stable.passes().len(),
                )
            },
        )
    };
    let passes = limits.base().get().max_line_reshape_passes;
    let expected = fresh(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        body,
        1_000_000,
        None,
        passes,
        None,
        None,
        |stable| {
            (
                stable.lines().fingerprint(),
                stable.candidate_steps(),
                stable.passes().len(),
            )
        },
    )
    .unwrap();
    assert!(expected.1 > 1 && expected.2 > 0);
    let mut exact = BookV2BodyLineBudget::new(expected.1, expected.2 as u16);
    assert_eq!(run(&mut exact, None).unwrap(), expected);
    assert_eq!((exact.remaining_steps(), exact.remaining_passes()), (0, 0));
    assert_eq!(exact.candidate_steps(), expected.1);
    let mut short = BookV2BodyLineBudget::new(expected.1 - 1, passes);
    assert!(run(&mut short, None).is_err());
    assert!(short.candidate_steps() > 0);
    assert!(
        short.reshape_passes() > 0,
        "failure must occur after starting a real reshape"
    );
    let before = (short.candidate_steps(), short.reshape_passes());
    assert!(run(&mut short, None).is_err());
    assert!(short.candidate_steps() >= before.0 && short.reshape_passes() >= before.1);
    assert!(short.candidate_steps() < expected.1);
    let mut exhausted = BookV2BodyLineBudget::new(1, passes);
    assert!(run(&mut exhausted, None).is_err());
    assert_eq!(
        (exhausted.candidate_steps(), exhausted.reshape_passes()),
        (1, 0)
    );
    assert!(run(&mut exhausted, None).is_err());
    assert_eq!(
        (exhausted.candidate_steps(), exhausted.reshape_passes()),
        (1, 0)
    );
    let mut no_pass = BookV2BodyLineBudget::new(1_000_000, 0);
    assert!(run(&mut no_pass, None).is_err());
    assert_eq!(no_pass.candidate_steps(), 0);

    // A consumer failure occurs after convergence and cannot refund its work.
    let mut consumer = BookV2BodyLineBudget::new(1_000_000, passes);
    let result = budgeted(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        body,
        None,
        &mut consumer,
        None,
        None,
        |_| Err::<(), _>("consumer failure"),
    )
    .unwrap();
    assert_eq!(result, Err("consumer failure"));
    assert_eq!(
        (consumer.candidate_steps(), consumer.reshape_passes()),
        (expected.1, expected.2 as u16)
    );

    // Width binding visits a paragraph before detecting an incorrect scalar count.
    let empty = [];
    let malformed = [Some(empty.as_slice())];
    let assignments = BookV2SourceWidthAssignments::new(&flow, &malformed).unwrap();
    let mut repeated = BookV2BodyLineBudget::new(3, passes);
    for expected in 1..=3 {
        assert!(run(&mut repeated, Some(&assignments)).is_err());
        assert_eq!(repeated.candidate_steps(), expected);
    }
    assert!(run(&mut repeated, Some(&assignments)).is_err());
    assert_eq!(
        (repeated.candidate_steps(), repeated.reshape_passes()),
        (3, 0)
    );

    // Identity rejection precedes traversal; it must not invent work.
    let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let assignments = BookV2SourceWidthAssignments::new(&foreign, &malformed).unwrap();
    let mut untouched = BookV2BodyLineBudget::new(100, passes);
    assert!(run(&mut untouched, Some(&assignments)).is_err());
    assert_eq!(untouched.candidate_steps(), 0);

    let (paragraph_width, unit_count, frame_records) = fresh(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        body,
        1_000_000,
        None,
        passes,
        None,
        None,
        |stable| {
            (
                stable.lines().paragraphs()[0].inline_size(),
                stable.lines().prepared().paragraphs()[0]
                    .items()
                    .unwrap()
                    .units()
                    .len(),
                stable.lines().frames().unwrap().record_charge(),
            )
        },
    )
    .unwrap();
    let widths = vec![paragraph_width; unit_count];
    let profiles = [Some(widths.as_slice())];
    let assigned = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
    let assigned_expected = fresh(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        body,
        1_000_000,
        None,
        passes,
        None,
        Some(&assigned),
        |stable| {
            (
                stable.lines().fingerprint(),
                stable.candidate_steps(),
                stable.passes().len(),
            )
        },
    )
    .unwrap();
    let mut assigned_exact = BookV2BodyLineBudget::new(assigned_expected.1, passes);
    assert_eq!(
        run(&mut assigned_exact, Some(&assigned)).unwrap(),
        assigned_expected
    );
    assert_eq!(assigned_exact.remaining_steps(), 0);

    // Failure occurs during initial shaping, before line-context reservations.
    let shape_records = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap()
    .output_records();
    // Rejected block ownership retains traversal; rejected table ownership retains
    // the existing conservative prepaid re-projection charge.
    let wrong_owner = [(typaxis_core::NodeId::new(999), width)];
    for table in [false, true] {
        let assigned = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
        let assigned = if table {
            assigned.with_root_table_widths(&wrong_owner)
        } else {
            assigned.with_block_widths(&wrong_owner)
        };
        let mut budget = BookV2BodyLineBudget::new(1_000_000, passes);
        assert!(run(&mut budget, Some(&assigned)).is_err());
        let charged = budget.candidate_steps();
        let records = shape_records.max(if table {
            2 * frame_records
        } else {
            frame_records + 1
        });
        assert_eq!(budget.record_charge(), records);
        assert!(charged > 0);
        assert!(run(&mut budget, Some(&assigned)).is_err());
        assert_eq!(budget.candidate_steps(), 2 * charged);
        let mut before_charge = BookV2BodyLineBudget::new(charged - 1, passes);
        assert!(run(&mut before_charge, Some(&assigned)).is_err());
        assert!(before_charge.candidate_steps() < charged);
        assert_eq!(before_charge.record_charge(), records);
        if table {
            assert_eq!(before_charge.candidate_steps(), 0);
        }
    }
    // Even an empty block-start profile traverses events before width binding
    // rejects the malformed paragraph. These accepted visits survive failure.
    let assigned = BookV2SourceWidthAssignments::new(&flow, &malformed)
        .unwrap()
        .with_block_starts(&[]);
    let mut budget = BookV2BodyLineBudget::new(1_000_000, passes);
    assert!(run(&mut budget, Some(&assigned)).is_err());
    let charged = budget.candidate_steps();
    assert!(charged > 1);
    assert!(run(&mut budget, Some(&assigned)).is_err());
    assert_eq!(budget.candidate_steps(), 2 * charged);

    // A source-start profile fails only after validating and visiting its frame.
    let invalid_starts = vec![Length::from_raw(-1).unwrap(); unit_count];
    let starts = [Some(invalid_starts.as_slice())];
    let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles)
        .unwrap()
        .with_source_unit_starts(&starts)
        .unwrap();
    let mut starts_budget = BookV2BodyLineBudget::new(1_000_000, passes);
    assert!(run(&mut starts_budget, Some(&assignments)).is_err());
    let charged = starts_budget.candidate_steps();
    assert!(charged > 1);
    assert!(run(&mut starts_budget, Some(&assignments)).is_err());
    assert_eq!(starts_budget.candidate_steps(), 2 * charged);
    assert_eq!(starts_budget.reshape_passes(), 0);
}

#[test]
fn book_v2_body_line_budget_driver_retains_failures_and_preserves_success() {
    use crate::book_v2_resources::{
        with_budgeted_book_v2_pdf, with_converged_book_v2_pdf, BookV2ConvergenceError,
        BookV2PdfConvergenceBudget,
    };
    let root = Root::new();
    let limits = limits();
    let input = input(&root, "Result", None, &limits);
    let expected =
        with_converged_book_v2_pdf(&input, &limits, MODE, 1_000_000, |pdf, observation| {
            (pdf.bytes().to_vec(), observation)
        })
        .unwrap();
    let mut budget = BookV2PdfConvergenceBudget::new(&limits, 1_000_000);
    let actual =
        with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut budget, |pdf, observation| {
            (pdf.bytes().to_vec(), observation)
        })
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(budget.observation(), actual.1);
    let mut changed_base = limits.base().get().clone();
    changed_base.max_fragments -= 1;
    let changed = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(changed_base).unwrap(),
        *limits.extension().get(),
    )
    .unwrap();
    assert!(matches!(
        with_budgeted_book_v2_pdf(&input, &changed, MODE, &mut budget, |_, _| ()),
        Err(BookV2ConvergenceError::Identity)
    ));
    assert_eq!(budget.observation(), actual.1);

    // Reach an actual line selection error after the driver has prepared page frames.
    let mut data = wide_data("Result");
    // The semantic container consumes 4 + 5 raw units of indentation.
    data["page_masters"]["masters"][0]["body"]["width"] = 10.into();
    let narrow_root = Root::new();
    let narrow = prepared(&narrow_root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(narrow.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(narrow.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(narrow.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, narrow.resources(), &limits).unwrap();
    let mut pre_work = 0;
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut pre_work,
        1_000_000,
        0,
        0,
    )
    .unwrap();
    assert!(!plan.requires_width_reflow());
    pre_work += flow
        .paragraphs()
        .iter()
        .map(|p| 1 + p.items().len() as u64)
        .sum::<u64>();
    let mut direct =
        BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
    assert!(budgeted(
        &policy,
        &flow,
        narrow.resources(),
        &bindings,
        &limits,
        MODE,
        plan.measurement_body(),
        None,
        &mut direct,
        Some(&plan),
        None,
        |_| ()
    )
    .is_err());
    assert!(
        direct.candidate_steps() > 0,
        "must reach charged line selection"
    );
    let mut failed = BookV2PdfConvergenceBudget::new(&limits, 1_000_000);
    let mut first = 0;
    for attempt in 1..=2 {
        let error =
            with_budgeted_book_v2_pdf(&narrow, &limits, MODE, &mut failed, |_, _| ()).unwrap_err();
        assert!(
            matches!(
                error,
                BookV2ConvergenceError::Stage {
                    stage: "line feedback",
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(failed.observation().work_steps() > 0);
        assert_eq!(failed.observation().candidate_passes(), 0);
        if attempt == 1 {
            first = failed.observation().work_steps();
            assert_eq!(first, pre_work + direct.candidate_steps());
        }
        assert_eq!(failed.observation().work_steps(), attempt * first);
    }
    assert!(first > 0);
    assert_eq!(failed.observation().work_steps(), 2 * first);
    let mut limited = BookV2PdfConvergenceBudget::new(&limits, first);
    assert!(with_budgeted_book_v2_pdf(&narrow, &limits, MODE, &mut limited, |_, _| ()).is_err());
    let charged = limited.observation().work_steps();
    assert_eq!(charged, first);
    assert!(with_budgeted_book_v2_pdf(&narrow, &limits, MODE, &mut limited, |_, _| ()).is_err());
    assert_eq!(limited.observation().work_steps(), charged);
}

#[test]
fn book_v2_line_variant_budget_retains_failed_capture_and_retries() {
    check_seed_budget("Result Result", None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_line_variant_budget_retains_original_harano_capture() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_seed_budget("本文", Some(&font));
}
fn check_seed_budget(text: &str, font: Option<&[u8]>) {
    use typaxis_layout::book_v2::{
        prepare_book_v2_body_line_variant_seed as fresh_seed,
        prepare_budgeted_book_v2_body_line_variant_seed as seed, BookV2LineVariantBudget,
    };
    let root = Root::new();
    let limits = limits();
    let input = input(&root, text, font, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow, &mut 0, 1_000_000, 0, 0,
    )
    .unwrap();
    let frame = plan.measurement_body();
    let passes = limits.base().get().max_line_reshape_passes;
    let expected = fresh_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        frame,
        1_000_000,
        0,
        None,
        passes,
        Some(&plan),
        None,
    )
    .unwrap();
    let run = |budget: &mut BookV2LineVariantBudget, prior| {
        seed(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            MODE,
            frame,
            budget,
            prior,
            None,
            Some(&plan),
            None,
        )
    };
    let mut exact = BookV2LineVariantBudget::new(expected.work_steps(), expected.reshape_passes());
    let actual = run(&mut exact, 0).unwrap();
    assert_eq!(actual.fingerprint(), expected.fingerprint());
    assert_eq!(actual.record_charge(), expected.record_charge());
    assert_eq!(exact.record_charge(), expected.record_charge());
    assert_eq!(actual.work_steps(), expected.work_steps());
    assert_eq!(actual.reshape_passes(), expected.reshape_passes());
    assert_eq!((exact.remaining_work(), exact.remaining_passes()), (0, 0));
    assert!(run(&mut exact, 0).is_err());
    assert_eq!(exact.work_steps(), expected.work_steps());
    let mut repeated =
        BookV2LineVariantBudget::new(2 * expected.work_steps(), 2 * expected.reshape_passes());
    for n in 1..=2 {
        let next = run(&mut repeated, 0).unwrap();
        assert_eq!(next.work_steps(), expected.work_steps());
        assert_eq!(repeated.work_steps(), n * expected.work_steps());
    }
    let mut short = BookV2LineVariantBudget::new(expected.work_steps() - 1, passes);
    assert!(run(&mut short, 0).is_err());
    assert_eq!(short.work_steps(), expected.work_steps() - 1);
    assert_eq!(short.reshape_passes(), expected.reshape_passes());
    assert!(short.record_charge() > 0 && short.record_charge() <= expected.record_charge());
    assert!(run(&mut short, 0).is_err());
    assert_eq!(short.work_steps(), expected.work_steps() - 1);
    let mut line = BookV2LineVariantBudget::new(1, passes);
    assert!(run(&mut line, 0).is_err());
    assert_eq!((line.work_steps(), line.reshape_passes()), (1, 0));
    assert!(run(&mut line, 0).is_err());
    assert_eq!(line.work_steps(), 1);
    let mut prior_rejected = BookV2LineVariantBudget::new(1_000_000, passes);
    assert!(run(&mut prior_rejected, limits.base().get().max_fragments).is_err());
    assert_eq!(
        (prior_rejected.work_steps(), prior_rejected.reshape_passes()),
        (0, 0)
    );
    assert_eq!(
        prior_rejected.record_charge(),
        limits.base().get().max_fragments
    );
    // Capacity for the seed context is checked after actual stable convergence.
    // Rejecting that allocation must retain both convergence and capture work.
    let captured = expected.contexts().paragraphs().iter()
        .map(|p| p.ends().len() as u64 + 2).sum::<u64>() + 1;
    assert_eq!(expected.record_charge(), expected.source_record_charge()
        + expected.retained_record_charge() + captured);
    let construction = expected.source_record_charge() + expected.retained_record_charge();
    for n in 1..=2 {
        let mut capture = BookV2LineVariantBudget::new(1_000_000, passes);
        assert!(run(&mut capture, limits.base().get().max_fragments - construction - 1).is_err());
        assert_eq!(capture.work_steps(), expected.work_steps());
        assert_eq!(capture.reshape_passes(), expected.reshape_passes());
        let records = capture.record_charge();
        assert!(run(&mut capture, 0).is_err());
        assert!(capture.record_charge() >= records);
        assert_eq!(capture.work_steps(), expected.work_steps(), "retry {n} must not invent convergence work");
    }
    let profiles = vec![None; flow.paragraphs().len()];
    let widths = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
    let sibling = expected
        .prepare_with_source_widths(&widths, 1_000_000, expected.record_charge(), passes)
        .unwrap();
    let mut allowance =
        BookV2LineVariantBudget::new(sibling.work_steps(), sibling.reshape_passes());
    let observed = expected
        .prepare_budgeted_with_source_widths(&widths, &mut allowance, expected.record_charge())
        .unwrap();
    assert_eq!(observed.fingerprint(), sibling.fingerprint());
    assert_eq!(observed.record_charge(), sibling.record_charge());
    assert_eq!(allowance.record_charge(), sibling.record_charge());
    assert_eq!(allowance.work_steps(), sibling.work_steps());
    assert_eq!(allowance.reshape_passes(), sibling.reshape_passes());
    let foreign_flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let foreign = BookV2SourceWidthAssignments::new(&foreign_flow, &profiles).unwrap();
    let mut allowance = BookV2LineVariantBudget::new(1_000_000, passes);
    assert!(expected
        .prepare_budgeted_with_source_widths(&foreign, &mut allowance, 0)
        .is_err());
    assert_eq!((allowance.work_steps(), allowance.reshape_passes()), (0, 0));
    let empty = [];
    let profiles = [Some(empty.as_slice())];
    let bad = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
    for n in 1..=2 {
        assert!(expected
            .prepare_budgeted_with_source_widths(&bad, &mut allowance, 0)
            .is_err());
        assert_eq!(allowance.work_steps(), n);
        assert_eq!(allowance.reshape_passes(), 0);
    }
}

#[test]
fn book_v2_line_replay_budget_retains_partial_graphs_and_retries() {
    check_replay_budget("Result Result", None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_line_replay_budget_retains_original_harano_graphs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_replay_budget("本文の再構築を繰り返す", Some(&font));
}
fn check_replay_budget(text: &str, font: Option<&[u8]>) {
    use typaxis_layout::book_v2::{
        prepare_book_v2_body_line_variant_seed as fresh_seed, BookV2LineVariantBudget,
    };
    let root = Root::new();
    let limits = limits();
    let input = input(&root, text, font, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow, &mut 0, 1_000_000, 0, 0,
    )
    .unwrap();
    let frame = plan.measurement_body();
    let passes = limits.base().get().max_line_reshape_passes;
    let expected = fresh_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        frame,
        1_000_000,
        0,
        None,
        passes,
        Some(&plan),
        None,
    )
    .unwrap();

    use typaxis_layout::book_v2::{
        with_budgeted_rebuilt_book_v2_body_line_variant as replay,
        with_budgeted_rebuilt_book_v2_body_line_variants as replay_set,
        with_rebuilt_book_v2_body_line_variant as old,
        with_rebuilt_book_v2_body_line_variants as old_set,
    };
    let baseline = old(&expected, 1_000_000, 0, |v| {
        (v.lines().fingerprint(), v.work_steps(), v.record_charge())
    })
    .unwrap();
    let run = |budget: &mut BookV2LineVariantBudget, prior| {
        replay(&expected, budget, prior, |v| {
            (v.lines().fingerprint(), v.work_steps(), v.record_charge())
        })
    };
    let graph = old(&expected, 1_000_000, 0, |v| v.footnotes().record_charge()).unwrap();
    let replay_reservation = graph + expected.contexts().paragraphs().len() as u64 + 1;
    let mut exact = BookV2LineVariantBudget::new(baseline.1, 0);
    assert_eq!(run(&mut exact, 0).unwrap(), baseline);
    assert_eq!(exact.record_charge(), baseline.2);
    assert_eq!((exact.remaining_work(), exact.reshape_passes()), (0, 0));
    assert!(run(&mut exact, 0).is_err());
    assert_eq!(exact.work_steps(), baseline.1);
    assert_eq!(exact.record_charge(), baseline.2 + replay_reservation);
    let mut short = BookV2LineVariantBudget::new(baseline.1 - 1, 0);
    assert!(run(&mut short, 0).is_err());
    assert_eq!(short.work_steps(), baseline.1 - 1);
    assert_eq!(short.record_charge(), baseline.2);
    assert!(run(&mut short, 0).is_err());
    assert_eq!(short.work_steps(), baseline.1 - 1);
    assert_eq!(short.record_charge(), baseline.2 + replay_reservation);
    let mut partial = BookV2LineVariantBudget::new(baseline.1 / 2, 0);
    assert!(run(&mut partial, 0).is_err());
    assert!(
        partial.work_steps() > 1,
        "must retain actual candidate work"
    );
    assert_eq!(partial.record_charge(), baseline.2);
    let before = partial.work_steps();
    assert!(run(&mut partial, 0).is_err());
    assert!(partial.work_steps() >= before && partial.work_steps() <= baseline.1 / 2);
    let mut no_records = BookV2LineVariantBudget::new(1_000_000, 0);
    assert!(run(&mut no_records, limits.base().get().max_fragments).is_err());
    assert_eq!(no_records.work_steps(), 0);
    assert_eq!(
        no_records.record_charge(),
        limits.base().get().max_fragments
    );
    let mut consumer = BookV2LineVariantBudget::new(1_000_000, 0);
    assert_eq!(
        replay(&expected, &mut consumer, 0, |_| Err::<(), _>("consumer")).unwrap(),
        Err("consumer")
    );
    assert_eq!(consumer.work_steps(), baseline.1);
    let seeds = [&expected, &expected];
    let all = old_set(&seeds, 1_000_000, 0, |v| {
        (
            v.fingerprint(),
            v.work_steps(),
            v.record_charge(),
            v.variants()
                .iter()
                .map(|v| v.lines().fingerprint())
                .collect::<Vec<_>>(),
        )
    })
    .unwrap();
    assert_eq!(all.3, vec![baseline.0, baseline.0]);
    let run_set = |budget: &mut BookV2LineVariantBudget, prior| {
        replay_set(&seeds, budget, prior, |v| {
            (
                v.fingerprint(),
                v.work_steps(),
                v.record_charge(),
                v.variants()
                    .iter()
                    .map(|v| v.lines().fingerprint())
                    .collect::<Vec<_>>(),
            )
        })
    };
    let mut exact = BookV2LineVariantBudget::new(all.1, 0);
    assert_eq!(run_set(&mut exact, 0).unwrap(), all);
    assert_eq!(exact.record_charge(), all.2);
    assert_eq!((exact.remaining_work(), exact.reshape_passes()), (0, 0));
    let mut repeated = BookV2LineVariantBudget::new(2 * all.1, 0);
    let set_reservation = seeds.len() as u64 * (graph + expected.contexts().paragraphs().len() as u64 + 5) + 1;
    let set_verifications = 2 * seeds.len() as u64 * flow.source_record_charge();
    for n in 1..=2 {
        let next = run_set(&mut repeated, 0).unwrap();
        assert_eq!((&next.0, next.1, &next.3), (&all.0, all.1, &all.3));
        assert_eq!(next.2, all.2 + (n - 1) * (set_reservation + set_verifications));
        assert_eq!(repeated.work_steps(), n * all.1);
    }
    let mut short = BookV2LineVariantBudget::new(all.1 - 1, 0);
    assert!(run_set(&mut short, 0).is_err());
    assert_eq!(short.work_steps(), all.1 - 1);
    assert_eq!(short.record_charge(), all.2);
    // Allow the first graph and only part of the second graph's candidate work.
    let cap = all.1 - baseline.1 / 2 - 3;
    let mut partial = BookV2LineVariantBudget::new(cap, 0);
    assert!(run_set(&mut partial, 0).is_err());
    assert!(
        partial.work_steps() > all.1 - baseline.1,
        "lost completed first graph or partial second graph: total={}, single={}, cap={}, charged={} ,", all.1, baseline.1, cap, partial.work_steps()
    );
    assert_eq!(partial.record_charge(), all.2);
    let charged = partial.work_steps();
    assert!(run_set(&mut partial, 0).is_err());
    assert!(partial.work_steps() >= charged && partial.work_steps() <= cap);
    let mut consumer = BookV2LineVariantBudget::new(1_000_000, 0);
    assert_eq!(
        replay_set(&seeds, &mut consumer, 0, |_| Err::<(), _>("consumer")).unwrap(),
        Err("consumer")
    );
    assert_eq!(consumer.work_steps(), all.1);
    assert_eq!(consumer.record_charge(), all.2);
    let mut records = BookV2LineVariantBudget::new(1_000_000, 0);
    assert!(run_set(&mut records, limits.base().get().max_fragments).is_err());
    assert_eq!(records.record_charge(), limits.base().get().max_fragments);
    let inspection = records.work_steps();
    assert!(inspection > 0 && inspection < all.1);
    assert!(run_set(&mut records, limits.base().get().max_fragments).is_err());
    assert_eq!(records.work_steps(), 2 * inspection);
    let mut empty = BookV2LineVariantBudget::new(1_000_000, 0);
    assert!(replay_set(&[], &mut empty, 0, |_| ()).is_err());
    assert_eq!(empty.work_steps(), 0);
    let foreign_flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let foreign = fresh_seed(
        &policy,
        &foreign_flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        frame,
        1_000_000,
        0,
        None,
        passes,
        Some(&plan),
        None,
    )
    .unwrap();
    let mut identity = BookV2LineVariantBudget::new(1_000_000, 0);
    assert!(replay_set(&[&expected, &foreign], &mut identity, 0, |_| ()).is_err());
    assert!(identity.work_steps() > 0 && identity.work_steps() < all.1);
    assert_eq!(identity.reshape_passes(), 0);
    assert_eq!(identity.record_charge(), expected.record_charge());
    // One command owner can retain seed convergence and replay without refunding
    // seed work or spending another reshape permit for replay.
    let mut shared = BookV2LineVariantBudget::new(expected.work_steps() + baseline.1, passes);
    let seed = typaxis_layout::book_v2::prepare_budgeted_book_v2_body_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        frame,
        &mut shared,
        0,
        None,
        Some(&plan),
        None,
    )
    .unwrap();
    let seed_passes = shared.reshape_passes();
    assert_eq!(
        replay(&seed, &mut shared, 0, |v| v.work_steps()).unwrap(),
        baseline.1
    );
    assert_eq!(shared.record_charge(), baseline.2);
    assert_eq!(shared.remaining_work(), 0);
    assert_eq!(shared.reshape_passes(), seed_passes);
}

#[test]
fn book_v2_page_search_budget_retains_failed_pass_work_and_retries() {
    use crate::book_v2_resources::{with_budgeted_book_v2_pdf, BookV2PdfConvergenceBudget};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements,
    };
    let root = Root::new();
    let limits = limits();
    let mut data = wide_data("Result");
    data["page_masters"]["masters"][0]["body"]["height"] = 1.into();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut pre_work = 0;
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut pre_work,
        1_000_000,
        0,
        0,
    )
    .unwrap();
    assert!(!plan.requires_width_reflow());
    pre_work += flow
        .paragraphs()
        .iter()
        .map(|p| 1 + p.items().len() as u64)
        .sum::<u64>();
    let mut line_budget =
        BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
    let (work, records, passes) = budgeted(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        plan.measurement_body(),
        None,
        &mut line_budget,
        Some(&plan),
        None,
        |lines| {
            let prior = plan.record_charge() + lines.footnotes().record_charge();
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(lines.lines(), None, lines.footnotes(), &limits, prior)
                    .unwrap(),
                &limits,
            )
            .unwrap();
            let mut search = prepare_book_v2_table_body_search(
                &measured,
                &limits,
                1_000_000,
                measured.record_charge(),
            )
            .unwrap();
            let mut passes = 0;
            assert!(search
                .select_stable_mixed_pages_counted(2, &mut passes)
                .is_err());
            assert_eq!(passes, 1);
            assert!(search.work_steps() > 0);
            (search.work_steps(), search.record_charge(), passes)
        },
    )
    .unwrap();
    let expected_work = pre_work + line_budget.candidate_steps() + work;
    let mut owner = BookV2PdfConvergenceBudget::new(&limits, 1_000_000);
    for attempt in 1..=2u16 {
        let error = with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut owner, |_, _| {
            panic!("invalid page reached PDF")
        })
        .unwrap_err();
        assert!(
            matches!(
                error,
                crate::book_v2_resources::BookV2ConvergenceError::Stage {
                    stage: "page stability",
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(
            owner.observation().work_steps(),
            u64::from(attempt) * expected_work
        );
        assert_eq!(owner.observation().page_passes(), attempt * passes);
        assert_eq!(
            owner.observation().line_reshape_passes(),
            attempt * line_budget.reshape_passes()
        );
        assert!(owner.observation().record_charge() >= records);
        assert_eq!(owner.observation().candidate_passes(), 0);
    }
    let mut exact = BookV2PdfConvergenceBudget::new(&limits, expected_work);
    assert!(with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut exact, |_, _| ()).is_err());
    assert_eq!(exact.observation().work_steps(), expected_work);
    assert_eq!(exact.observation().page_passes(), passes);
    assert!(with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut exact, |_, _| ()).is_err());
    assert_eq!(exact.observation().work_steps(), expected_work);
    assert_eq!(exact.observation().page_passes(), passes);
}
