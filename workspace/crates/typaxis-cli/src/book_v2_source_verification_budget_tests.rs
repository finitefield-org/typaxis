use super::*;
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, prepare_book_v2_inline_items_with_native_context,
    prepare_book_v2_inline_items_with_source_budget_counted,
    prepare_budgeted_book_v2_body_line_variant_seed,
    with_budgeted_book_v2_body_lines_with_source_widths,
    with_budgeted_rebuilt_book_v2_body_line_variant,
    with_budgeted_rebuilt_book_v2_body_line_variants, BookV2BodyLineBudget,
    BookV2LineVariantBudget,
};
use typaxis_layout::{ProductionBodyReshapeError, ProductionInlinePreparationErrorKind};
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_shaping::book_v2::shape_book_v2_authored_text_with_source_budget_counted;
use typaxis_syntax::book_v2::BookV2SourceVerificationBudget;

#[test]
fn book_v2_source_verification_budget_spans_shaping_lines_replay_and_driver() {
    check(None, "Result");
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_source_verification_budget_spans_original_harano_paths() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font), "本文");
}

fn check(font: Option<&[u8]>, text: &str) {
    let limits = limits();
    let root = Root::new();
    let data = super::body_line_budget::wide_data(text);
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let charge = 1
        + flow.events().len() as u64
        + flow.paragraphs().len() as u64
        + flow
            .paragraphs()
            .iter()
            .map(|p| p.items().len() as u64)
            .sum::<u64>();
    assert_eq!(charge, flow.source_record_charge());
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let epoch = bindings.epoch();
    let mode = JapaneseLineBreakMode::Normal;
    let shape =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, epoch, None)
            .unwrap();
    let plain = prepare_book_v2_inline_items_with_native_context(
        &flow,
        &shape,
        input.resources(),
        &bindings,
        &limits,
        mode,
        None,
    )
    .unwrap();
    for available in 0..charge {
        let mut source = BookV2SourceVerificationBudget::new(7, 7 + available);
        let mut output = u64::MAX;
        let error = shape_book_v2_authored_text_with_source_budget_counted(
            &policy,
            &flow,
            input.resources(),
            &limits,
            epoch,
            None,
            &mut source,
            &mut output,
        )
        .unwrap_err();
        assert_eq!(error.kind, ProductionTextShapeErrorKind::OutputLimit);
        assert_eq!((source.record_charge(), output), (7 + available, 0));
        let mut source =
            BookV2SourceVerificationBudget::new(7, 7 + shape.output_records() + available);
        source.include_retained_records(shape.output_records());
        let error = prepare_book_v2_inline_items_with_source_budget_counted(
            &flow,
            &shape,
            input.resources(),
            &bindings,
            &limits,
            mode,
            None,
            &mut source,
            &mut output,
        )
        .err()
        .unwrap();
        assert_eq!(error.kind, ProductionInlinePreparationErrorKind::UnitLimit);
        assert_eq!((source.record_charge(), output), (7 + available, 0));
    }
    let mut source = BookV2SourceVerificationBudget::new(7, limits.base().get().max_fragments);
    let mut output = 0;
    let counted = shape_book_v2_authored_text_with_source_budget_counted(
        &policy,
        &flow,
        input.resources(),
        &limits,
        epoch,
        None,
        &mut source,
        &mut output,
    )
    .unwrap();
    assert_eq!(counted.fingerprint(), shape.fingerprint());
    assert_eq!(output, shape.output_records());
    assert_eq!(source.record_charge(), 7 + charge);
    let inlines = prepare_book_v2_inline_items_with_source_budget_counted(
        &flow,
        &counted,
        input.resources(),
        &bindings,
        &limits,
        mode,
        None,
        &mut source,
        &mut output,
    )
    .unwrap();
    assert_eq!(inlines.fingerprint(), plain.fingerprint());
    assert_eq!(source.record_charge(), 7 + 2 * charge);
    let error = shape_book_v2_authored_text_with_source_budget_counted(
        &policy,
        &flow,
        input.resources(),
        &limits,
        [0; 32],
        None,
        &mut source,
        &mut output,
    )
    .unwrap_err();
    assert_eq!(error.kind, ProductionTextShapeErrorKind::ReceiptMismatch);
    assert_eq!((source.record_charge(), output), (7 + 3 * charge, 0));

    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut 0,
        100_000_000,
        0,
        0,
    )
    .unwrap();
    let passes = limits.base().get().max_line_reshape_passes;
    let mut budget = BookV2BodyLineBudget::new_with_source_records(
        100_000_000,
        passes,
        7,
        limits.base().get().max_fragments,
    );
    let mut expected = None;
    let mut previous = 7;
    for _ in 0..2 {
        let observed = with_budgeted_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            mode,
            plan.measurement_body(),
            None,
            &mut budget,
            Some(&plan),
            None,
            |stable| {
                (
                    stable.lines().fingerprint(),
                    stable.passes().len(),
                    stable.source_record_charge(),
                )
            },
        )
        .unwrap();
        let steps = 2 * (1 + observed.1 as u64) * charge;
        assert_eq!(observed.2, previous + steps);
        assert_eq!(budget.source_record_charge(), observed.2);
        if let Some(fingerprint) = expected {
            assert_eq!(observed.0, fingerprint);
        }
        expected = Some(observed.0);
        previous = observed.2;
    }
    for available in 0..charge {
        let mut budget =
            BookV2BodyLineBudget::new_with_source_records(100_000_000, passes, 7, 7 + available);
        let error = with_budgeted_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            mode,
            plan.measurement_body(),
            None,
            &mut budget,
            Some(&plan),
            None,
            |_| (),
        )
        .unwrap_err();
        assert!(
            matches!(error, ProductionBodyReshapeError::Shape(e) if e.kind == ProductionTextShapeErrorKind::OutputLimit)
        );
        assert_eq!(
            (
                budget.source_record_charge(),
                budget.record_charge(),
                budget.candidate_steps(),
                budget.reshape_passes()
            ),
            (7 + available, 0, 0, 0)
        );
        assert!(with_budgeted_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            mode,
            plan.measurement_body(),
            None,
            &mut budget,
            Some(&plan),
            None,
            |_| (),
        )
        .is_err());
        assert_eq!(budget.source_record_charge(), 7 + available);
    }
    let mut seed_budget = BookV2LineVariantBudget::new(100_000_000, passes);
    let seed = prepare_budgeted_book_v2_body_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        mode,
        plan.measurement_body(),
        &mut seed_budget,
        7,
        None,
        Some(&plan),
        None,
    )
    .unwrap();
    assert_eq!(
        seed.source_record_charge(),
        7 + 2 * (1 + u64::from(seed.reshape_passes())) * charge
    );
    let mut replay = BookV2LineVariantBudget::new(100_000_000, 0);
    let one = with_budgeted_rebuilt_book_v2_body_line_variant(&seed, &mut replay, 0, |v| {
        (v.lines().fingerprint(), v.record_charge())
    })
    .unwrap();
    assert_eq!(one.0, seed.fingerprint());
    assert_eq!(one.1, replay.record_charge());
    let prior = one.1;
    let two = with_budgeted_rebuilt_book_v2_body_line_variant(&seed, &mut replay, prior, |v| {
        v.record_charge()
    })
    .unwrap();
    assert!(two >= prior + 2 * charge);
    assert_eq!(two, replay.record_charge());
    let mut set = BookV2LineVariantBudget::new(100_000_000, 0);
    let many =
        with_budgeted_rebuilt_book_v2_body_line_variants(&[&seed, &seed], &mut set, 0, |v| {
            assert_eq!(v.variants().len(), 2);
            v.record_charge()
        })
        .unwrap();
    assert_eq!(many, set.record_charge());
    assert!(many >= seed.record_charge() + 4 * charge);
    check_driver(font, text, false, charge);
    check_driver(font, text, true, charge);
}

fn check_driver(font: Option<&[u8]>, text: &str, running_regions: bool, charge: u64) {
    use crate::book_v2_resources::{
        with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
    };
    let original = limits();
    let make = |root: &Root, limits: &M4EffectiveResourceLimits| {
        let data = if running_regions {
            super::page_regions::region_data(text)
        } else {
            super::body_line_budget::wide_data(text)
        };
        if let Some(font) = font {
            vector_tests::vector_input_with_font(root, data, limits, font, text.as_bytes())
        } else {
            prepared(root, data, text.as_bytes(), limits)
        }
    };
    let root = Root::new();
    let input = make(&root, &original);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut 0,
        100_000_000,
        charge,
        0,
    )
    .unwrap();
    let initial = nav.source_record_charge() + 2 * charge + plan.record_charge();
    for available in 0..charge {
        let mut base = original.base().get().clone();
        base.max_fragments = initial + available;
        let limits = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        let root = Root::new();
        let input = make(&root, &limits);
        let mut budget = BookV2PdfConvergenceBudget::new(&limits, 100_000_000);
        let error = with_budgeted_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| panic!("source verification limit must precede PDF construction"),
        )
        .err()
        .unwrap();
        let BookV2ConvergenceError::Stage { stage, source } = error else {
            panic!("{error:?}")
        };
        assert_eq!(stage, "line feedback");
        let error = source.downcast_ref::<ProductionBodyReshapeError>().unwrap();
        assert!(
            matches!(error, ProductionBodyReshapeError::Shape(e) if e.kind == ProductionTextShapeErrorKind::OutputLimit)
        );
        let observed = budget.observation();
        assert_eq!(observed.record_charge(), initial + available);
        assert_eq!(
            (
                observed.line_reshape_passes(),
                observed.page_passes(),
                observed.output_charge()
            ),
            (0, 0, 0)
        );
        assert!(with_budgeted_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| ()
        )
        .is_err());
        assert_eq!(budget.observation(), observed);
    }
}
