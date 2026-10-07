use super::*;
use crate::book_v2_resources::{
    converged_pdf::page_region_driver::attach, with_budgeted_book_v2_pdf,
    BookV2PdfConvergenceBudget,
};
use typaxis_display_list::book_v2::BookV2MathDisplayBuilder;

#[test]
fn book_v2_region_failure_budget_retains_partial_attachment() {
    verify(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_region_failure_budget_retains_original_harano_attachment() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    verify(Some(&font));
}

fn verify(font: Option<&[u8]>) {
    let root = Root::new();
    let base = limits();
    let mut caps = base.base().get().clone();
    caps.max_line_reshape_passes = 128;
    caps.max_layout_passes = 128;
    let limits = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
        base.extension().get().clone(),
    )
    .unwrap();
    let text = if font.is_some() {
        "本文の柱"
    } else {
        "Result"
    };
    let data = pdf_data(text, false, false);
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let mode = typaxis_linebreak::JapaneseLineBreakMode::Normal;
    with_converged_book_v2_pdf(&input, &limits, mode, 100_000_000, |pdf, observed| {
        let display = pdf
            .navigation()
            .source()
            .source()
            .source()
            .source()
            .display();
        let navigation = display
            .source()
            .source()
            .flow()
            .lines()
            .prepared()
            .source_flow()
            .navigation();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let fresh = || {
            BookV2MathDisplayBuilder::new(
                display.source(),
                input.resources(),
                &limits,
                100_000_000,
                0,
                0,
            )
            .unwrap()
            .build_body()
            .unwrap()
        };
        let start = fresh().work_steps();
        let initial_records = fresh().record_charge();
        let constructor_slots = input.resources().fonts().len() as u64 * 2 + 1;
        let mut passes = 0;
        let mut records = 0;
        let mut work = 0;
        let joined = attach(
            fresh(),
            &policy,
            navigation,
            &limits,
            mode,
            100_000_000,
            &mut passes,
            &mut records,
            &mut work,
        )
        .unwrap();
        assert_eq!(joined.fingerprint(), display.fingerprint());
        assert_eq!(joined.work_steps(), work);
        assert_eq!(joined.record_charge(), records);
        assert!(passes > 2);
        let prefix = observed.work_steps() - pdf.work_steps();
        let body_passes = observed.line_reshape_passes() - passes;
        let end = work;
        // Independently prepare the first running region, stopping immediately
        // before its first candidate. The attachment must retain those exact
        // source/shape/inline/line reservations even without its callback.
        let source = input.body().styled();
        let epoch = display
            .source()
            .source()
            .flow()
            .lines()
            .prepared()
            .shaped()
            .binding_epoch();
        let builder = typaxis_display_list::book_v2::BookV2PageRegionDisplayBuilder::new_counted(
            source,
            input.resources(),
            &limits,
            epoch,
            initial_records,
            start,
            100_000_000,
            &mut 0,
            &mut 0,
        )
        .unwrap();
        let mut prefix_region_work = builder.work_steps();
        let selected = select_book_v2_page_master(
            source,
            0,
            display.source().source().geometry().pages()[0]
                .selection()
                .named_page(),
            &mut prefix_region_work,
            100_000_000,
        )
        .unwrap();
        let source_prefix = builder.record_charge() + 1;
        let region_flow = region_flow(selected, Kind::Header, navigation, source_prefix).unwrap();
        let before_source_work = prefix_region_work;
        prefix_region_work += region_flow.record_charge() - source_prefix;
        // Carry a real body display with caller history close to the cap. The
        // first region then rejects either its owner or its paragraph atomically.
        // Compare independently collected source counters with attachment, both
        // with enough work and with no allowance for the accepted source prefix.
        let source_slots = region_flow.record_charge() - source_prefix;
        let body_slots = initial_records - display.source().record_charge();
        let cap = limits.base().get().max_fragments;
        for remaining in 0..source_slots {
            let prior = cap - remaining;
            let mut expected_records = 0;
            let source_error =
                typaxis_syntax::book_v2::prepare_book_v2_page_region_text_flow_counted(
                    selected,
                    Kind::Header,
                    navigation,
                    prior,
                    &mut expected_records,
                )
                .err()
                .unwrap();
            let accepted = expected_records - prior;
            for maximum_work in [100_000_000, before_source_work] {
                let near_cap = BookV2MathDisplayBuilder::new(
                    display.source(),
                    input.resources(),
                    &limits,
                    100_000_000,
                    prior - constructor_slots - 1 - body_slots,
                    0,
                )
                .unwrap()
                .build_body()
                .unwrap();
                assert_eq!(near_cap.record_charge(), prior - constructor_slots - 1);
                let mut records = u64::MAX;
                let mut work = u64::MAX;
                let mut passes = 0;
                let failure = attach(
                    near_cap,
                    &policy,
                    navigation,
                    &limits,
                    mode,
                    maximum_work,
                    &mut passes,
                    &mut records,
                    &mut work,
                )
                .err()
                .unwrap();
                let crate::book_v2_resources::BookV2ConvergenceError::Stage { stage, source } =
                    failure
                else {
                    panic!("source failure must retain its precedence");
                };
                assert_eq!(stage, "page-region source");
                assert_eq!(
                    source.downcast_ref::<typaxis_syntax::ProductionFlowError>(),
                    Some(&source_error)
                );
                assert_eq!(records, expected_records);
                assert_eq!(
                    work,
                    before_source_work
                        + if maximum_work > before_source_work {
                            accepted
                        } else {
                            0
                        }
                );
                assert_eq!(passes, 0);
            }
        }
        let mut first_region = typaxis_layout::book_v2::BookV2PageRegionLineBudget::new(0, 128);
        assert!(
            typaxis_layout::book_v2::with_budgeted_book_v2_page_region_lines(
                &policy,
                &region_flow,
                input.resources(),
                &limits,
                epoch,
                mode,
                selected,
                &mut first_region,
                region_flow.record_charge(),
                |_| ()
            )
            .is_err()
        );
        assert!(first_region.record_charge() > region_flow.record_charge());
        let mut first_records = 0;
        let mut first_work = 0;
        let mut first_passes = 0;
        let failure = attach(
            fresh(),
            &policy,
            navigation,
            &limits,
            mode,
            prefix_region_work,
            &mut first_passes,
            &mut first_records,
            &mut first_work,
        )
        .err()
        .unwrap();
        assert!(format!("{failure:?}").contains("page-region lines"));
        assert_eq!(first_records, first_region.record_charge());
        assert_eq!(first_work, prefix_region_work);
        assert_eq!(first_passes, 0);
        let mut command = BookV2PdfConvergenceBudget::new(&limits, prefix + prefix_region_work);
        assert!(with_budgeted_book_v2_pdf(&input, &limits, mode, &mut command, |_, _| ()).is_err());
        assert_eq!(command.observation().record_charge(), first_records);
        assert_eq!(command.observation().work_steps(), prefix + first_work);
        let mut failures = std::collections::BTreeSet::new();
        // Sample the whole multi-page attachment, plus the final merge boundary.
        // Earlier completed regions must survive a failure in a later region.
        for cap in (1..16).map(|n| start + (end - start) * n / 16).chain([
            end - 1,
            start,
            start + constructor_slots - 1,
        ]) {
            let mut passes = 0;
            let mut records = 0;
            let mut work = 0;
            let error = match attach(
                fresh(),
                &policy,
                navigation,
                &limits,
                mode,
                cap,
                &mut passes,
                &mut records,
                &mut work,
            ) {
                Ok(_) => panic!("short attachment unexpectedly succeeded"),
                Err(e) => e,
            };
            let message = format!("{error:?}");
            if message.contains("page-region join") {
                failures.insert("join");
            }
            if message.contains("page-region display") {
                failures.insert("display");
            }
            if message.contains("page-region lines") && !message.contains("page-region display") {
                failures.insert("lines");
            }
            assert!(
                work >= start && work <= cap,
                "{message}: {start}/{work}/{cap}"
            );
            if cap < start + constructor_slots {
                assert_eq!(work, start, "rejected work reservation is not consumed");
                assert_eq!(records, initial_records + constructor_slots);
                assert_eq!(passes, 0);
            }
            let mut owner = BookV2PdfConvergenceBudget::new(&limits, prefix + cap);
            let failure = with_budgeted_book_v2_pdf(&input, &limits, mode, &mut owner, |_, _| {
                panic!("failed region emitted PDF")
            })
            .unwrap_err();
            assert_eq!(format!("{failure:?}"), message);
            let actual = owner.observation();
            assert_eq!(actual.work_steps(), prefix + work);
            assert_eq!(actual.record_charge(), records);
            assert_eq!(actual.spool_charge(), display.source().spool_charge());
            assert_eq!(actual.output_charge(), 0);
            assert_eq!(actual.candidate_passes(), 0);
            assert_eq!(actual.line_reshape_passes(), body_passes + passes);
            assert!(
                with_budgeted_book_v2_pdf(&input, &limits, mode, &mut owner, |_, _| ()).is_err()
            );
            assert!(owner.observation().work_steps() >= actual.work_steps());
            assert!(owner.observation().work_steps() <= prefix + cap);
            assert!(owner.observation().record_charge() >= records);
        }
        assert_eq!(failures, ["display", "join", "lines"].into_iter().collect());
        let exact = attach(
            fresh(),
            &policy,
            navigation,
            &limits,
            mode,
            end,
            &mut 0,
            &mut 0,
            &mut 0,
        )
        .unwrap();
        assert_eq!(exact.fingerprint(), joined.fingerprint());
        assert_eq!(exact.work_steps(), joined.work_steps());
        assert_eq!(exact.record_charge(), joined.record_charge());
    })
    .unwrap();
}
