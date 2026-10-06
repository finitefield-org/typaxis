use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_syntax::{book_v2::prepare_book_v2_navigation_counted, BookNavigationSyntaxError};

#[test]
fn book_v2_navigation_text_limits_preserve_driver_history_before_layout() {
    check_text(None, "Result");
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_navigation_text_limits_preserve_original_harano_driver_history() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_text(Some(&font), "本文");
}
fn check_text(font: Option<&[u8]>, text: &str) {
    let make = |root: &Root, caps: &M4EffectiveResourceLimits| {
        let mut data = source_frame(source_data(text));
        data["metadata"]["title"] = "t".repeat(128).into();
        if let Some(font) = font {
            vector_tests::vector_input_with_font(root, data, caps, font, text.as_bytes())
        } else {
            prepared(root, data, text.as_bytes(), caps)
        }
    };
    let original = limits();
    let root = Root::new();
    let input = make(&root, &original);
    let reference = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let charge = reference.source_record_charge();
    let baseline = input.body().styled().body().retained_text_bytes();
    for maximum in [baseline + 127, reference.retained_text_bytes() - 1] {
        let mut caps = original.base().get().clone();
        caps.max_text_bytes = maximum;
        caps.max_text_buffer_bytes = maximum as u32;
        caps.max_shaping_context_bytes = maximum as u32;
        let bounded = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(caps).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        let root = Root::new();
        let input = make(&root, &bounded);
        let expected = prepare_book_v2_navigation(input.body().styled()).unwrap_err();
        assert_eq!(
            expected.kind(),
            typaxis_syntax::BookNavigationSyntaxErrorKind::TextAggregateLimit
        );
        let mut budget = BookV2PdfConvergenceBudget::new(&bounded, 100_000_000);
        for attempt in 1..=2 {
            let error = with_budgeted_book_v2_pdf(
                &input,
                &bounded,
                JapaneseLineBreakMode::Normal,
                &mut budget,
                |_, _| panic!("text quota failure emitted PDF"),
            )
            .unwrap_err();
            let BookV2ConvergenceError::Stage { stage, source } = error else {
                panic!("{error:?}");
            };
            assert_eq!(stage, "navigation");
            let source = source.downcast_ref::<BookNavigationSyntaxError>().unwrap();
            assert_eq!(source, &expected);
            let observed = budget.observation();
            assert_eq!(observed.record_charge(), attempt * charge);
            assert_eq!(
                (
                    observed.work_steps(),
                    observed.spool_charge(),
                    observed.output_charge(),
                    observed.candidate_passes(),
                    observed.line_reshape_passes()
                ),
                (0, 0, 0, 0, 0)
            );
        }
    }
}

#[test]
fn book_v2_navigation_constructor_bounds_precede_lines_and_keep_driver_history() {
    check(None, "Result");
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_navigation_constructor_bounds_keep_original_harano_driver_history() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font), "本文");
}
fn check(font: Option<&[u8]>, text: &str) {
    let make = |root: &Root, limits: &M4EffectiveResourceLimits| {
        let data = source_frame(source_data(text));
        if let Some(font) = font {
            vector_tests::vector_input_with_font(root, data, limits, font, text.as_bytes())
        } else {
            prepared(root, data, text.as_bytes(), limits)
        }
    };
    let original = limits();
    let root = Root::new();
    let input = make(&root, &original);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let charge = nav.source_record_charge();
    assert!(charge > nav.languages().len() as u64);
    for available in [1, charge - 1, charge, 2 * charge - 1] {
        let mut caps = original.base().get().clone();
        caps.max_fragments = available;
        let bounded = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(caps).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        let root = Root::new();
        let input = make(&root, &bounded);
        let mut budget = BookV2PdfConvergenceBudget::new(&bounded, 100_000_000);
        let mut syntax_history = 0;
        let syntax = prepare_book_v2_navigation_counted(
            input.body().styled(),
            0,
            available,
            &mut syntax_history,
        );
        for attempt in 0..2 {
            let before = budget.observation();
            let error = with_budgeted_book_v2_pdf(
                &input,
                &bounded,
                JapaneseLineBreakMode::Normal,
                &mut budget,
                |_, _| panic!("insufficient initial source room emitted PDF"),
            )
            .unwrap_err();
            let observed = budget.observation();
            assert!(observed.record_charge() >= before.record_charge());
            assert!(observed.record_charge() <= available);
            assert_eq!(observed.output_charge(), 0);
            if syntax.is_err() {
                assert!(matches!(error, BookV2ConvergenceError::Limit("records")));
                if attempt == 0 {
                    assert_eq!(observed.record_charge(), syntax_history);
                }
                assert_eq!(observed.work_steps(), 0);
                assert_eq!(observed.spool_charge(), 0);
                assert_eq!(observed.candidate_passes(), 0);
                assert_eq!(observed.line_reshape_passes(), 0);
            }
        }
    }
    let mut bad = source_frame(source_data(text));
    bad["metadata"]["title"] = "bad\u{0000}title".into();
    let root = Root::new();
    let invalid = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, bad, &original, font, text.as_bytes())
    } else {
        prepared(&root, bad, text.as_bytes(), &original)
    };
    let mut budget = BookV2PdfConvergenceBudget::new(&original, 100_000_000);
    for attempt in 1..=2 {
        let error = with_budgeted_book_v2_pdf(
            &invalid,
            &original,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| panic!("invalid navigation emitted PDF"),
        )
        .unwrap_err();
        let BookV2ConvergenceError::Stage { stage, source } = error else {
            panic!("{error:?}")
        };
        assert_eq!(stage, "navigation");
        let source = source.downcast_ref::<BookNavigationSyntaxError>().unwrap();
        assert_eq!(source.pointer().to_string(), "/metadata/title");
        let observed = budget.observation();
        assert_eq!(observed.record_charge(), attempt * charge);
        assert_eq!(
            (
                observed.work_steps(),
                observed.spool_charge(),
                observed.output_charge()
            ),
            (0, 0, 0)
        );
    }
}
