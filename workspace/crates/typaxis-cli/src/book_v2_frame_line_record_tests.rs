use super::*;
use typaxis_layout::book_v2::{
    prepare_book_v2_body_inline_frames, prepare_book_v2_body_inline_frames_counted,
};

#[test]
fn book_v2_frame_line_records_preserve_initial_and_binding_failures() {
    check("Result", None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_frame_line_records_preserve_original_harano_failures() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check("本文", Some(&bytes));
}
fn check(text: &str, font: Option<&[u8]>) {
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
    let body = plan.measurement_body();
    let passes = limits.base().get().max_line_reshape_passes;
    let run = |budget: &mut BookV2BodyLineBudget,
               assignments: Option<&BookV2SourceWidthAssignments<'_, '_>>,
               page: bool| {
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
            page.then_some(&plan),
            assignments,
            |stable| {
                let prepared = stable.lines().prepared();
                let mut observed = u64::MAX;
                let frames =
                    prepare_book_v2_body_inline_frames_counted(prepared, body, &mut observed)
                        .unwrap();
                assert_eq!(observed, frames.record_charge());
                assert_eq!(
                    frames.fingerprint(),
                    prepare_book_v2_body_inline_frames(prepared, body)
                        .unwrap()
                        .fingerprint()
                );
                let base = observed;
                // The source container has horizontal padding; width 1 is rejected
                // after accepting the complete frame reservation.
                let narrow = Rect::new(
                    body.x(),
                    body.y(),
                    PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
                    body.height(),
                );
                let cause =
                    prepare_book_v2_body_inline_frames_counted(prepared, narrow, &mut observed)
                        .err()
                        .unwrap();
                assert_eq!(observed, base);
                assert_eq!(
                    cause,
                    prepare_book_v2_body_inline_frames(prepared, narrow)
                        .err()
                        .unwrap()
                );
                (
                    base,
                    prepared.paragraphs()[0].items().unwrap().units().len(),
                    stable.lines().paragraphs()[0].inline_size(),
                    stable.lines().output_records(),
                )
            },
        )
    };
    let mut full = BookV2BodyLineBudget::new(1_000_000, passes);
    let (frame_records, count, width, line_records) = run(&mut full, None, false).unwrap();
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
    assert!(full.record_charge() >= line_records);
    for page in [false, true] {
        let mut zero = BookV2BodyLineBudget::new(0, passes);
        assert!(run(&mut zero, None, page).is_err());
        // The first line candidate is rejected after paragraph and unit slots
        // have been reserved. No candidate or reshape was accepted.
        assert_eq!(
            zero.record_charge(),
            frame_records + flow.paragraphs().len() as u64 + count as u64
        );
        assert_eq!((zero.candidate_steps(), zero.reshape_passes()), (0, 0));
        let before = zero.record_charge();
        assert!(run(&mut zero, None, page).is_err());
        assert_eq!(zero.record_charge(), before);
    }
    let widths = vec![width; count];
    let profiles = [Some(widths.as_slice())];
    let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
    // Zero work rejects the first binding visit after reserving its vector;
    // one accepted visit reserves the complete width slice before text binding.
    for (work, extra) in [(0, 1), (1, 1 + count as u64)] {
        let mut budget = BookV2BodyLineBudget::new(work, passes);
        assert!(run(&mut budget, Some(&assignments), false).is_err());
        assert_eq!(
            budget.record_charge(),
            shape_records.max(frame_records + extra)
        );
        assert_eq!(budget.candidate_steps(), work);
    }
    let starts_values = vec![Length::from_raw(-1).unwrap(); count];
    let starts = [Some(starts_values.as_slice())];
    let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles)
        .unwrap()
        .with_source_unit_starts(&starts)
        .unwrap();
    for (work, extra) in [(0, 2), (1, 2 + count as u64), (1_000_000, 2 + count as u64)] {
        let mut budget = BookV2BodyLineBudget::new(work, passes);
        assert!(run(&mut budget, Some(&assignments), false).is_err());
        assert_eq!(
            budget.record_charge(),
            shape_records.max(frame_records + extra)
        );
        assert_eq!(budget.reshape_passes(), 0);
    }
    let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let assignments = BookV2SourceWidthAssignments::new(&foreign, &profiles).unwrap();
    let mut budget = BookV2BodyLineBudget::new(1_000_000, passes);
    assert!(run(&mut budget, Some(&assignments), false).is_err());
    assert_eq!(budget.record_charge(), shape_records.max(frame_records));
    assert_eq!(budget.candidate_steps(), 0);
}
