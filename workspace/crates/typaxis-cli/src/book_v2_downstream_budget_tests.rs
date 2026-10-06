use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_display_list::book_v2::BookV2MathDisplayBuilder;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements,
};
use typaxis_pdf::book_v2::BookV2PdfPipeline;

#[test]
fn book_v2_downstream_budget_retains_display_and_pdf_failures() {
    check("Result", None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_downstream_budget_retains_original_harano_failures() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check("本文", Some(&font));
}

fn check(text: &str, font: Option<&[u8]>) {
    let root = Root::new();
    let limits = limits();
    let input = input(&root, text, font, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut pre_work = 0;
    let maximum = 100_000_000;
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut pre_work,
        maximum,
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
        BookV2BodyLineBudget::new(maximum, limits.base().get().max_line_reshape_passes);
    budgeted(
        &policy, &flow, input.resources(), &bindings, &limits, MODE,
        plan.measurement_body(), None, &mut line_budget, Some(&plan), None,
        |lines| {
            // Construct the downstream owners independently of the command
            // driver, so a lost failure counter cannot hide in a self-comparison.
            let prefix = pre_work + lines.candidate_steps();
            let records = command_source_record_charge(&flow)
                + plan.record_charge() + lines.footnotes().record_charge();
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(lines.lines(), None, lines.footnotes(), &limits, records).unwrap(),
                &limits,
            ).unwrap();
            let mut search = prepare_book_v2_table_body_search(
                &measured, &limits, maximum, measured.record_charge(),
            ).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search.finalize_mixed_page_math(closure, &limits, plan.spool_charge()).unwrap();
            let display_builder = |cap| BookV2MathDisplayBuilder::new(
                &terminals, input.resources(), &limits, cap,
                terminals.record_charge(), terminals.work_steps(),
            ).unwrap();
            let mut builder = display_builder(maximum);
            let display = builder.build_body().unwrap();
            let pipeline = |cap| BookV2PdfPipeline::new(
                &display, 1, &limits, cap, display.record_charge(),
                terminals.spool_charge(), 0, display.work_steps(),
            ).unwrap();
            let mut complete = pipeline(maximum);
            let expected_pdf = complete.with_pdf(|pdf| pdf.bytes().to_vec()).unwrap();
            let final_work = complete.work_steps();
            assert!(final_work > display.work_steps() + 2);
            let verify_failure = |cap, expected_stage, work, records, spool, output| {
                let mut owner = BookV2PdfConvergenceBudget::new(&limits, prefix + cap);
                let error = with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut owner, |_, _| {
                    panic!("failed candidate reached the final consumer")
                }).unwrap_err();
                assert!(matches!(&error, BookV2ConvergenceError::Stage { stage, .. } if *stage == expected_stage), "{error:?}");
                assert!(std::error::Error::source(&error).is_some());
                let observed = owner.observation();
                assert_eq!(observed.work_steps(), prefix + work, "{error:?}");
                assert_eq!(observed.record_charge(), records, "{error:?}");
                assert_eq!(observed.spool_charge(), spool, "{error:?}");
                assert_eq!(observed.output_charge(), output, "{error:?}");
                assert_eq!(observed.page_passes(), 2);
                assert_eq!(observed.line_reshape_passes() as usize, lines.passes().len());
                assert_eq!(observed.candidate_passes(), 0);
                // Retrying cannot restore any accepted budget dimension.
                assert!(with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut owner, |_, _| ()).is_err());
                let retried = owner.observation();
                assert!(retried.work_steps() >= observed.work_steps());
                assert!(retried.work_steps() <= prefix + cap);
                assert!(retried.record_charge() >= observed.record_charge());
                assert!(retried.spool_charge() >= observed.spool_charge());
                assert!(retried.output_charge() >= observed.output_charge());
            };
            // Exercise early and late body-display failure, including records
            // reserved before projection completes and the math-terminal spool.
            for cap in [terminals.work_steps() + 1, display.work_steps() - 1] {
                let mut failed = display_builder(cap);
                assert!(failed.build_body().is_err());
                assert!(failed.work_steps() > search.work_steps());
                verify_failure(cap, "body display", failed.work_steps(), failed.record_charge(), terminals.spool_charge(), 0);
            }
            // Exercise resource selection, partial PDF construction, and final
            // assembly. The near-complete failure must retain real output bytes.
            for cap in [display.work_steps() + 1, (display.work_steps() + final_work) / 2, final_work - 1] {
                let mut failed = pipeline(cap);
                assert!(failed.with_pdf(|_| ()).is_err());
                if cap == final_work - 1 {
                    assert!(failed.spool_charge() > terminals.spool_charge());
                    assert!(failed.output_charge() > 0);
                }
                verify_failure(cap, "PDF", failed.work_steps(), failed.record_charge(), failed.spool_charge(), failed.output_charge());
            }
            // Success at the exact measured ceiling preserves bytes and charges,
            // including when the caller itself returns a failure value.
            let mut exact = BookV2PdfConvergenceBudget::new(&limits, prefix + final_work);
            let result = with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut exact, |pdf, observed| {
                assert_eq!(pdf.bytes(), expected_pdf);
                assert_eq!(observed.work_steps(), prefix + final_work);
                assert_eq!(observed.record_charge(), complete.record_charge());
                assert_eq!(observed.spool_charge(), complete.spool_charge());
                assert_eq!(observed.output_charge(), complete.output_charge());
                Err::<(), _>("consumer")
            }).unwrap();
            assert_eq!(result, Err("consumer"));
            assert_eq!(exact.observation().candidate_passes(), 1);
            assert_eq!(exact.observation().work_steps(), prefix + final_work);
        },
    ).unwrap();
}
