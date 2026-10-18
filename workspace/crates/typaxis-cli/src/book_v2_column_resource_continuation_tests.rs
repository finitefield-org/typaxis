//! Actual standalone resource callback accepted-prefix ownership controls.
use super::*;

fn charges(observed: BookV2PdfConvergenceObservation) -> [u64; 3] {
    [
        observed.record_charge(),
        observed.spool_charge(),
        observed.work_steps(),
    ]
}

#[test]
fn book_v2_column_resource_continuation_extra_build_retains_ok_and_application_error_prefixes() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    for fail_application in [false, true] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut accepted = [0; 3];
        let result: Result<Result<(), &'static str>, _> = resource_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            |bundle, loan, observed| {
                let before = charges(observed);
                assert_eq!(
                    [loan.record_charge(), loan.spool_charge(), loan.work_steps()],
                    before
                );
                let selection = loan.build().unwrap();
                assert!(std::ptr::eq(selection.display(), bundle.display()));
                assert_eq!(selection.fingerprint(), bundle.fonts().fingerprint());
                assert!(!selection.uses().is_empty());
                accepted = [loan.record_charge(), loan.spool_charge(), loan.work_steps()];
                assert!(accepted.iter().zip(before).all(|(a, b)| *a > b));
                // The before-fix executable lost exactly these accepted charges
                // when safe callback code replaced the original raw builder.
                assert_eq!(before, [3907, 17458, 17846]);
                assert_eq!(accepted, [3962, 24674, 18360]);
                if fail_application {
                    Err("continuation application sentinel")
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(
            result.unwrap(),
            if fail_application {
                Err("continuation application sentinel")
            } else {
                Ok(())
            }
        );
        assert_eq!(charges(budget.observation()), accepted);
        eprintln!(
            "resource continuation application_error={fail_application} accepted={accepted:?}"
        );
        let caller_before = accepted;
        resource_drive(&input, &caps, MODE, &mut budget, |bundle, _, observed| {
            assert_eq!(bundle.caller_before(), caller_before);
            assert!(charges(observed)
                .iter()
                .zip(caller_before)
                .all(|(a, b)| *a > b));
        })
        .unwrap();
        assert!(charges(budget.observation())
            .iter()
            .zip(accepted)
            .all(|(a, b)| *a > b));
    }
}

#[test]
fn book_v2_column_resource_continuation_forwards_original_complete_resource_chain() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut accepted = [0; 3];
    resource_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        |bundle, loan, observed| {
            let units = loan.reservation_sizes();
            assert!(units.iter().all(|n| *n > 0));
            let selection = loan.build().unwrap();
            let closures = loan.prepare_font_closures(&selection).unwrap();
            let stats = loan.prepare_cff_programs(&closures).unwrap();
            let programs = loan.write_font_programs(&closures).unwrap();
            let cids = loan.plan_cids(&programs).unwrap();
            let images = loan.select_images().unwrap();
            let rasters = loan.write_raster_programs(&images).unwrap();
            assert_eq!(selection.fingerprint(), bundle.fonts().fingerprint());
            assert_eq!(cids.fingerprint(), bundle.cids().fingerprint());
            assert_eq!(rasters.fingerprint(), bundle.rasters().fingerprint());
            assert!(std::ptr::eq(
                cids.source().source().selection().display(),
                bundle.display()
            ));
            assert!(std::ptr::eq(rasters.source().display(), bundle.display()));
            assert_eq!(loan.reservation_sizes(), units);
            assert_eq!(loan.cff_program_statistics(), stats);
            // Genuine TrueType source: zero evaluator observations are not CFF proof.
            assert_eq!(
                (
                    stats.operations_used,
                    stats.outline_segments_used,
                    stats.cached_glyphs
                ),
                (0, 0, 0)
            );
            accepted = [loan.record_charge(), loan.spool_charge(), loan.work_steps()];
            assert!(accepted.iter().zip(charges(observed)).all(|(a, b)| *a > b));
        },
    )
    .unwrap();
    assert_eq!(charges(budget.observation()), accepted);
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_resource_continuation_forgetting_borrow_keeps_original_owner() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut accepted = [0; 3];
    let result: Result<Result<(), &'static str>, _> =
        resource_drive(&input, &caps, MODE, &mut budget, |bundle, loan, _| {
            let selection = loan.build().unwrap();
            assert!(std::ptr::eq(selection.display(), bundle.display()));
            accepted = [loan.record_charge(), loan.spool_charge(), loan.work_steps()];
            std::mem::forget(loan);
            Err("forgotten reference sentinel")
        });
    assert_eq!(result.unwrap(), Err("forgotten reference sentinel"));
    assert_eq!(charges(budget.observation()), accepted);
}

#[test]
fn book_v2_column_resource_continuation_late_cap_errors_and_retry_keep_actual_prefixes() {
    let initial_caps = display_caps();
    let initial_root = Root::new();
    let data = plain_data("Pro", true, false);
    let initial_input = display_input(&initial_root, data.clone(), None, &initial_caps);
    let mut warm = BookV2ColumnPageBudget::new(&initial_caps, MAXIMUM);
    let mut baseline_before = [0; 3];
    let mut baseline_after = [0; 3];
    resource_drive(
        &initial_input,
        &initial_caps,
        MODE,
        &mut warm,
        |_, loan, observed| {
            baseline_before = charges(observed);
            loan.build().unwrap();
            baseline_after = [loan.record_charge(), loan.spool_charge(), loan.work_steps()];
        },
    )
    .unwrap();
    for axis in 0..3 {
        let cap = baseline_after[axis] - 1;
        let caps = match axis {
            0 => changed_caps(&initial_caps, |c| c.max_fragments = cap),
            1 => {
                let mut base = initial_caps.base().get().clone();
                base.max_spool_bytes = cap;
                let mut extension = initial_caps.extension().get().clone();
                extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(cap);
                M4EffectiveResourceLimits::new(
                    ValidatedResourceLimits::new(base).unwrap(),
                    extension,
                )
                .unwrap()
            }
            _ => initial_caps.clone(),
        };
        let maximum = if axis == 2 { cap } else { MAXIMUM };
        let root = Root::new();
        let input = display_input(&root, data.clone(), None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
        let mut delivered = false;
        let mut accepted = [0; 3];
        let result: Result<Result<(), &'static str>, _> =
            resource_drive(&input, &caps, MODE, &mut budget, |_, loan, observed| {
                delivered = true;
                assert_eq!(charges(observed), baseline_before);
                let expected = match axis {
                    0 => BookV2FontSelectionError::Records,
                    1 => BookV2FontSelectionError::Spool,
                    _ => BookV2FontSelectionError::Work,
                };
                assert!(matches!(loan.build(), Err(e) if e == expected));
                accepted = [loan.record_charge(), loan.spool_charge(), loan.work_steps()];
                assert!(accepted.iter().zip(baseline_before).all(|(a, b)| *a >= b));
                assert!(accepted[2] > baseline_before[2]);
                assert!(accepted[axis] <= cap);
                if axis == 2 {
                    assert_eq!(accepted, [baseline_after[0], baseline_after[1], cap]);
                }
                Err("late resource error sentinel")
            });
        assert!(delivered, "axis {axis}: {result:?}");
        assert_eq!(result.unwrap(), Err("late resource error sentinel"));
        assert_eq!(charges(budget.observation()), accepted);
        eprintln!("resource continuation late axis={axis} actual={accepted:?}");
        let mut redelivered = false;
        assert!(
            resource_drive(&input, &caps, MODE, &mut budget, |_, _, _| redelivered =
                true)
            .is_err()
        );
        assert!(!redelivered);
        assert!(charges(budget.observation())
            .iter()
            .zip(accepted)
            .all(|(a, b)| *a >= b));
    }
}
