use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_core::NodeId;
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_syntax::{ProductionFlowError, ProductionFlowErrorKind};

#[test]
fn book_v2_source_flow_records_survive_initial_and_candidate_driver_failures() {
    check_source_records(None, "Result");
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_source_flow_records_survive_original_harano_driver_failures() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_source_records(Some(&font), "本文");
}

fn source_input(
    root: &Root,
    limits: &M4EffectiveResourceLimits,
    font: Option<&[u8]>,
    text: &str,
    missing: bool,
) -> PreparedBookV2Resources {
    let mut data = source_data(text);
    let mut second = data["document"]["blocks"][0]["blocks"][0].clone();
    second["node_id"] = 4.into();
    second["children"][0]["node_id"] = 5.into();
    if missing {
        second["kind"] = "heading".into();
        second["level"] = 2.into();
        second["anchor_id"] = Value::Null;
    }
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(second);
    if let Some(font) = font {
        vector_tests::vector_input_with_font(root, data, limits, font, text.as_bytes())
    } else {
        prepared(root, data, text.as_bytes(), limits)
    }
}

fn bounded_records(
    original: &M4EffectiveResourceLimits,
    maximum: u64,
) -> M4EffectiveResourceLimits {
    let mut base = original.base().get().clone();
    base.max_fragments = maximum;
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        original.extension().get().clone(),
    )
    .unwrap()
}

fn assert_source_failure(
    error: BookV2ConvergenceError,
    stage: &str,
    kind: ProductionFlowErrorKind,
) -> NodeId {
    let BookV2ConvergenceError::Stage {
        stage: actual,
        source,
    } = error
    else {
        panic!("expected a typed source-stage failure: {error:?}")
    };
    assert_eq!(actual, stage);
    let cause = source.downcast_ref::<ProductionFlowError>().unwrap();
    assert_eq!(cause.kind, kind);
    cause.owner
}

fn check_source_records(font: Option<&[u8]>, text: &str) {
    let original = limits();
    // Flow owner, enclosing begin, first paragraph's five carriers, then the
    // failing paragraph's begin/paragraph/inline: ten accepted reservations.
    for maximum in [19, 1000] {
        let limits = bounded_records(&original, maximum);
        let root = Root::new();
        let input = source_input(&root, &limits, font, text, true);
        let mut budget = BookV2PdfConvergenceBudget::new(&limits, 100_000_000);
        for attempt in 1..=3 {
            let error = with_budgeted_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                &mut budget,
                |_, _| panic!("invalid source must not emit a PDF"),
            )
            .err()
            .unwrap();
            let missing = maximum == 1000 || attempt == 1;
            let owner = assert_source_failure(
                error,
                "source",
                if missing {
                    ProductionFlowErrorKind::MissingTextStyle
                } else {
                    ProductionFlowErrorKind::NodeLimit
                },
            );
            assert_eq!(
                owner,
                NodeId::new(if missing {
                    4
                } else if attempt == 2 {
                    5
                } else {
                    0
                })
            );
            let history = budget.observation();
            assert_eq!(history.record_charge(), (10 * attempt).min(maximum));
            assert_eq!(
                (
                    history.work_steps(),
                    history.spool_charge(),
                    history.output_charge(),
                    history.candidate_passes(),
                    history.page_passes(),
                    history.line_reshape_passes()
                ),
                (0, 0, 0, 0, 0, 0)
            );
        }
    }
    let root = Root::new();
    let input = source_input(&root, &original, font, text, false);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let source = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    assert_eq!(source.source_record_charge(), 13);
    let frames = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &source,
        &mut 0,
        100_000_000,
        13,
        0,
    )
    .unwrap();
    let initial = 13 + frames.record_charge();
    // Every rejected position within the actual candidate-label constructor.
    for available in 0..13 {
        let maximum = initial + available;
        let limits = bounded_records(&original, maximum);
        let root = Root::new();
        let input = source_input(&root, &limits, font, text, false);
        let mut budget = BookV2PdfConvergenceBudget::new(&limits, 100_000_000);
        let error = with_budgeted_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| panic!("record bound must stop before a PDF"),
        )
        .err()
        .unwrap();
        assert_source_failure(error, "source labels", ProductionFlowErrorKind::NodeLimit);
        let history = budget.observation();
        assert_eq!(history.record_charge(), maximum);
        assert_eq!(
            (
                history.spool_charge(),
                history.output_charge(),
                history.page_passes(),
                history.line_reshape_passes(),
                history.candidate_passes()
            ),
            (frames.spool_charge(), 0, 0, 0, 0)
        );
        let error = with_budgeted_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| (),
        )
        .err()
        .unwrap();
        assert_source_failure(error, "source", ProductionFlowErrorKind::NodeLimit);
        assert_eq!(budget.observation(), history);
    }
}
