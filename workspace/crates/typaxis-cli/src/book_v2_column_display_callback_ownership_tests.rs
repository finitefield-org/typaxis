//! Genuine standalone Display callback ownership controls.
use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_column_display as display_drive, BookV2PdfConvergenceObservation,
};

fn quartet(observed: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [
        observed.record_charge(),
        observed.spool_charge(),
        observed.output_charge(),
        observed.work_steps(),
    ]
}
fn continuation_type_name<T: ?Sized>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}
#[test]
fn book_v2_column_display_callback_extra_build_and_callback_results_keep_original_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    for application_error in [false, true] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
        let (mut original, mut extra) = ([0; 4], [0; 4]);
        let result = display_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            |display, loan, observed| {
                assert!(continuation_type_name(loan).contains("BookV2ColumnDisplayContinuation"));
                original = quartet(observed);
                assert_eq!(
                    [loan.record_charge(), loan.work_steps()],
                    [original[0], original[3]]
                );
                let again = loan.build().unwrap();
                assert_eq!(again.fingerprint(), display.fingerprint());
                extra = [
                    loan.record_charge(),
                    observed.spool_charge(),
                    observed.output_charge(),
                    loan.work_steps(),
                ];
                assert!(extra[0] > original[0] && extra[3] > original[3]);
                if application_error {
                    Err("Display callback sentinel")
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(
            result.unwrap(),
            if application_error {
                Err("Display callback sentinel")
            } else {
                Ok(())
            }
        );
        let actual = quartet(budget.observation());
        assert_eq!(actual, extra);
        eprintln!(
            "DISPLAY_CALLBACK_OWNERSHIP_AFTER {}",
            serde_json::json!({"application_error":application_error,"original":original,"original_after_extra_build":extra,"caller_after":actual})
        );
    }
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_display_callback_failed_build_forgotten_borrow_and_retry_keep_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
    let second_work = display_drive(&input, &caps, MODE, &mut warm, |_, loan, _| {
        loan.build().unwrap();
        loan.work_steps()
    })
    .unwrap();
    let cap = second_work - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, cap);
    let mut accepted = [0; 4];
    let result = display_drive(&input, &caps, MODE, &mut budget, |_, loan, observed| {
        let failure = loan
            .build()
            .err()
            .expect("late work cap unexpectedly delivered another display");
        assert!(
            matches!(failure, BookV2MathDisplayError::WorkLimit(_)),
            "{failure:?}"
        );
        accepted = [
            loan.record_charge(),
            observed.spool_charge(),
            observed.output_charge(),
            loan.work_steps(),
        ];
        assert!(accepted[0] > observed.record_charge());
        assert_eq!(accepted[3], cap);
        // Only this borrowed continuation reference is forgotten. The driver
        // still owns its original raw builder and rereads it after normal return.
        std::mem::forget(loan);
        Err::<(), _>("forgotten Display borrow sentinel")
    });
    assert_eq!(result.unwrap(), Err("forgotten Display borrow sentinel"));
    assert_eq!(quartet(budget.observation()), accepted);
    let mut called = false;
    let retry = display_drive(&input, &caps, MODE, &mut budget, |_, _, _| called = true);
    assert!(retry.is_err());
    assert!(!called);
    let retained = quartet(budget.observation());
    assert!(retained.into_iter().zip(accepted).all(|(a, b)| a >= b));
    assert_eq!(retained[3], cap);
    eprintln!(
        "DISPLAY_CALLBACK_OWNERSHIP_AFTER_FAILURE {}",
        serde_json::json!({"accepted_late_failure":accepted,"after_retry":retained,"callback_returned_application_error":true,"forgotten_value":"borrowed continuation reference"})
    );
}
