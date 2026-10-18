//! Actual standalone Relations callback ownership controls.
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
fn book_v2_column_relations_callback_extra_build_and_callback_results_keep_original_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    for application_error in [false, true] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let (mut before, mut extra) = ([0; 4], [0; 4]);
        let result = relation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, loan, _| {
            before = quartet!(loan);
            let again = loan.build().unwrap();
            assert_eq!(again.fingerprint(), bundle.relations().fingerprint());
            extra = quartet!(loan);
            drop(again);
            if application_error {
                Err("relation callback sentinel")
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(
            result,
            if application_error {
                Err("relation callback sentinel")
            } else {
                Ok(())
            }
        );
        let after = paint_observation(budget.observation());
        for axis in [0, 1, 3] {
            assert!(extra[axis] > before[axis], "axis{axis}");
        }
        assert_eq!(extra[2], before[2], "Relations adds no output or credit");
        assert_eq!(after, extra);
        eprintln!("RELATIONS_OWNERSHIP_AFTER application_error={application_error} before={before:?} extra={extra:?} caller={after:?}");
    }
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_relations_callback_failed_build_forgotten_borrow_and_retry_keep_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    let mut warm_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut warm = [0; 4];
    relation_drive(&input, &caps, MODE, &mut warm_budget, 4, |_, loan, _| {
        drop(loan.build().unwrap());
        warm = quartet!(loan);
    })
    .unwrap();
    let maximum = warm[3] - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
    let (mut before, mut failed) = ([0; 4], [0; 4]);
    let result = relation_drive(&input, &caps, MODE, &mut budget, 4, |_, loan, _| {
        before = quartet!(loan);
        assert!(matches!(loan.build(), Err(RelationError::Work)));
        failed = quartet!(loan);
        // This forgets only the borrowed reference; the owner remains with the
        // driver and is reread after a normal Result return.
        std::mem::forget(loan);
        Err::<(), _>("late relation work sentinel")
    })
    .unwrap();
    assert_eq!(result, Err("late relation work sentinel"));
    assert_eq!(&failed[..3], &warm[..3]);
    assert!(failed[3] > before[3] && failed[3] <= maximum);
    assert_eq!(failed[2], before[2]);
    let after = paint_observation(budget.observation());
    assert_eq!(after, failed);
    let mut delivered = false;
    assert!(
        relation_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| delivered =
            true)
        .is_err()
    );
    assert!(!delivered);
    let retry = paint_observation(budget.observation());
    for axis in [0, 1, 3] {
        assert!(retry[axis] >= after[axis]);
    }
    assert_eq!(
        retry[2], after[2],
        "this work refusal reaches no new marked settlement"
    );
    assert!(retry[3] <= maximum);
    eprintln!("RELATIONS_OWNERSHIP_LATE before={before:?} full={warm:?} failed={failed:?} caller={after:?} retry={retry:?}");
}
