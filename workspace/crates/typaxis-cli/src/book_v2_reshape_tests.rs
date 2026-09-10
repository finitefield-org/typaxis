use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_layout::book_v2::{bind_book_v2_vectors, with_converged_book_v2_body_lines};
use typaxis_layout::{ProductionBodyReshapeError, ProductionPlacedInline};
use typaxis_linebreak::{BreakError, JapaneseLineBreakMode};
fn body() -> Rect {
    let size = PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap();
    Rect::new(Length::ZERO, Length::ZERO, size, size)
}
#[test]
fn book_v2_reshape_publishes_only_actual_stable_lines_and_charges_all_candidates() {
    let root = Root::new();
    let limits = limits();
    let input = prepared(
        &root,
        source_data("Result Proof Result"),
        b"Result Proof Result",
        &limits,
    );
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut called = 0;
    let (steps, fingerprint) = with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body(),
        1_000_000,
        |stable| {
            called += 1;
            assert!(stable.passes().len() >= 2);
            assert!(!stable.passes()[0].is_stable());
            assert!(stable.passes().last().unwrap().is_stable());
            for pair in stable.passes().windows(2) {
                assert_eq!(pair[0].output(), pair[1].input());
                assert_eq!(pair[0].pass_index() + 1, pair[1].pass_index());
            }
            let lines = stable.lines();
            assert!(lines
                .prepared()
                .shaped()
                .line_context_fingerprint()
                .is_some());
            lines
                .frames()
                .unwrap()
                .verify(lines.prepared(), body())
                .unwrap();
            assert!(stable.candidate_steps() > lines.candidate_steps());
            let text: String = lines
                .paragraphs()
                .iter()
                .flat_map(|p| p.lines())
                .flat_map(|l| l.items())
                .map(|i| match i {
                    ProductionPlacedInline::Text(t) => t.utf8(),
                    _ => panic!("text fixture"),
                })
                .collect();
            assert_eq!(text, "Result Proof Result");
            (stable.candidate_steps(), lines.fingerprint())
        },
    )
    .unwrap();
    assert_eq!(called, 1);
    let exact = with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body(),
        steps,
        |s| {
            assert_eq!(s.candidate_steps(), steps);
            s.lines().fingerprint()
        },
    )
    .unwrap();
    assert_eq!(exact, fingerprint);
    let mut called = false;
    let result = with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body(),
        steps - 1,
        |_| {
            called = true;
        },
    );
    assert!(result.is_err());
    assert!(!called);
}
#[test]
fn book_v2_reshape_pass_limit_never_publishes_unstable_shape() {
    let root = Root::new();
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits {
            max_line_reshape_passes: 1,
            ..ResourceLimits::default()
        })
        .unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    let input = prepared(
        &root,
        source_data("Result Proof Result"),
        b"Result Proof Result",
        &limits,
    );
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut called = false;
    let result = with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body(),
        1_000_000,
        |_| {
            called = true;
        },
    );
    assert!(matches!(
        result,
        Err(ProductionBodyReshapeError::Feedback(
            BreakError::IterationLimit
        ))
    ));
    assert!(!called);
}
