//! Real standalone Paints callback ownership controls.
use super::*;

macro_rules! quartet {
    ($builder:expr) => {
        [
            $builder.record_charge(),
            $builder.spool_charge(),
            $builder.output_charge(),
            $builder.work_steps(),
        ]
    };
}

#[test]
fn book_v2_column_paints_callback_extra_build_and_callback_results_keep_original_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    for application_error in [false, true] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let (mut before, mut extra) = ([0; 4], [0; 4]);
        let result = paint_drive(&input, &caps, MODE, &mut budget, 4, |bundle, loan, _| {
            assert!(!bundle.images().commands().is_empty());
            assert!(!bundle.images().bytes().is_empty());
            before = quartet!(loan);
            let again = loan.build().unwrap();
            assert_eq!(again.fingerprint(), bundle.images().fingerprint());
            extra = quartet!(loan);
            drop(again);
            if application_error {
                Err("paint callback sentinel")
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(
            result,
            if application_error {
                Err("paint callback sentinel")
            } else {
                Ok(())
            }
        );
        let after = paint_observation(budget.observation());
        for axis in 0..4 {
            assert!(extra[axis] > before[axis], "axis{axis}");
        }
        assert_eq!(after, extra);
        eprintln!("PAINTS_OWNERSHIP_AFTER application_error={application_error} before={before:?} extra={extra:?} caller={after:?}");
    }
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_paints_callback_failed_build_forgotten_borrow_and_retry_keep_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let mut warm_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut warm = [0; 4];
    paint_drive(&input, &caps, MODE, &mut warm_budget, 4, |_, loan, _| {
        drop(loan.build().unwrap());
        warm = quartet!(loan);
    })
    .unwrap();
    let maximum = warm[3] - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
    let (mut before, mut failed) = ([0; 4], [0; 4]);
    let result = paint_drive(&input, &caps, MODE, &mut budget, 4, |_, loan, _| {
        before = quartet!(loan);
        assert!(matches!(
            loan.build(),
            Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)
        ));
        failed = quartet!(loan);
        // Only the borrowed reference is forgotten; the driver still owns
        // the original builder and rereads it after normal callback return.
        std::mem::forget(loan);
        Err::<(), _>("late paint work sentinel")
    })
    .unwrap();
    assert_eq!(result, Err("late paint work sentinel"));
    assert_eq!(&failed[..3], &warm[..3]);
    assert!(failed[3] > before[3] && failed[3] <= maximum);
    let after = paint_observation(budget.observation());
    assert_eq!(after, failed);
    let mut delivered = false;
    assert!(
        paint_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| delivered =
            true)
        .is_err()
    );
    assert!(!delivered);
    let retry = paint_observation(budget.observation());
    for axis in 0..4 {
        assert!(retry[axis] >= after[axis]);
    }
    assert!(retry[3] <= maximum);
    eprintln!("PAINTS_OWNERSHIP_LATE before={before:?} full={warm:?} failed={failed:?} caller={after:?} retry={retry:?}");
}
