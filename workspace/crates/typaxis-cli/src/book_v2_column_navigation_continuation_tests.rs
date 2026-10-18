//! Standalone Navigation callback ownership on authentic Column relations.
use super::*;

#[test]
fn book_v2_column_navigation_continuation_extra_build_retains_ok_and_application_error_prefixes() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, description_data(), None, &caps);
    for application_error in [false, true] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut accepted = [0; 4];
        let result: Result<Result<(), &'static str>, _> = navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, loan, observed| {
                let before = paint_observation(observed);
                assert_eq!(
                    [
                        loan.record_charge(),
                        loan.spool_charge(),
                        loan.output_charge(),
                        loan.work_steps()
                    ],
                    before
                );
                let sizes = loan.reservation_sizes();
                assert!(sizes[0] > 0);
                let structure = loan.build().unwrap();
                assert_eq!(loan.reservation_sizes(), sizes);
                assert!(std::ptr::eq(structure.source(), bundle.relations()));
                assert_eq!(structure.fingerprint(), bundle.navigation().fingerprint());
                accepted = [
                    loan.record_charge(),
                    loan.spool_charge(),
                    loan.output_charge(),
                    loan.work_steps(),
                ];
                for axis in [0, 1, 3] {
                    assert!(accepted[axis] > before[axis]);
                }
                assert_eq!(accepted[2], before[2]);
                assert_eq!(accepted[2], bundle.relations().output_charge());
                // The genuine frozen before executable discarded the extra
                // R/S/W after callback code replaced its original raw builder.
                assert_eq!(before, [3778, 82663, 13098, 44341]);
                assert_eq!(accepted, [3815, 84543, 13098, 44709]);
                if application_error {
                    Err("navigation continuation sentinel")
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(
            result.unwrap(),
            if application_error {
                Err("navigation continuation sentinel")
            } else {
                Ok(())
            }
        );
        assert_eq!(paint_observation(budget.observation()), accepted);
        eprintln!(
            "navigation continuation application_error={application_error} accepted={accepted:?}"
        );
        navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, loan, observed| {
                assert_eq!(bundle.caller_before(), accepted);
                let after = paint_observation(observed);
                for axis in [0, 1, 3] {
                    assert!(after[axis] > accepted[axis]);
                }
                assert_eq!(loan.output_charge(), bundle.relations().output_charge());
            },
        )
        .unwrap();
        let after = paint_observation(budget.observation());
        for axis in [0, 1, 3] {
            assert!(after[axis] > accepted[axis]);
        }
    }
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_navigation_continuation_forgetting_borrow_keeps_original_builder_and_output() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, description_data(), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut accepted = [0; 4];
    let result: Result<Result<(), &'static str>, _> =
        navigation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, loan, _| {
            let structure = loan.build().unwrap();
            assert!(std::ptr::eq(structure.source(), bundle.relations()));
            accepted = [
                loan.record_charge(),
                loan.spool_charge(),
                loan.output_charge(),
                loan.work_steps(),
            ];
            assert_eq!(accepted[2], bundle.relations().output_charge());
            std::mem::forget(loan);
            Err("forgotten navigation reference")
        });
    assert_eq!(result.unwrap(), Err("forgotten navigation reference"));
    assert_eq!(paint_observation(budget.observation()), accepted);
}

#[test]
fn book_v2_column_navigation_continuation_late_record_spool_work_errors_keep_prefix_and_settled_output(
) {
    let baseline_caps = display_caps();
    let root = Root::new();
    let data = description_data();
    let input = display_input(&root, data.clone(), None, &baseline_caps);
    let mut warm = BookV2ColumnPageBudget::new(&baseline_caps, MAXIMUM);
    let mut baseline_before = [0; 4];
    let mut baseline_after = [0; 4];
    navigation_drive(
        &input,
        &baseline_caps,
        MODE,
        &mut warm,
        4,
        |_, loan, observed| {
            baseline_before = paint_observation(observed);
            loan.build().unwrap();
            baseline_after = [
                loan.record_charge(),
                loan.spool_charge(),
                loan.output_charge(),
                loan.work_steps(),
            ];
        },
    )
    .unwrap();
    for axis in [0, 1, 3] {
        let cap = baseline_after[axis] - 1;
        let caps = match axis {
            0 => changed_caps(&baseline_caps, |c| c.max_fragments = cap),
            1 => {
                let mut base = baseline_caps.base().get().clone();
                base.max_spool_bytes = cap;
                let mut extension = baseline_caps.extension().get().clone();
                extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(cap);
                M4EffectiveResourceLimits::new(
                    ValidatedResourceLimits::new(base).unwrap(),
                    extension,
                )
                .unwrap()
            }
            _ => baseline_caps.clone(),
        };
        let maximum = if axis == 3 { cap } else { MAXIMUM };
        let root = Root::new();
        let input = display_input(&root, data.clone(), None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
        let mut accepted = [0; 4];
        let mut delivered = false;
        let result: Result<Result<(), &'static str>, _> = navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, loan, observed| {
                delivered = true;
                assert_eq!(paint_observation(observed), baseline_before);
                let expected = NavigationError::Resource(match axis {
                    0 => typaxis_pdf::book_v2::BookV2FontStreamError::Records,
                    1 => typaxis_pdf::book_v2::BookV2FontStreamError::Spool,
                    _ => typaxis_pdf::book_v2::BookV2FontStreamError::Work,
                });
                assert!(matches!(loan.build(),Err(e) if e==expected));
                accepted = [
                    loan.record_charge(),
                    loan.spool_charge(),
                    loan.output_charge(),
                    loan.work_steps(),
                ];
                for i in [0, 1, 3] {
                    assert!(accepted[i] >= baseline_before[i]);
                }
                assert!(accepted[3] > baseline_before[3]);
                assert!(accepted[axis] <= cap);
                assert_eq!(accepted[2], baseline_before[2]);
                assert_eq!(accepted[2], bundle.relations().output_charge());
                if axis == 3 {
                    assert_eq!(
                        accepted,
                        [
                            baseline_after[0],
                            baseline_after[1],
                            baseline_before[2],
                            cap
                        ]
                    );
                }
                let repeat_before = accepted;
                assert!(matches!(loan.build(), Err(e) if e == expected));
                let repeated = [
                    loan.record_charge(),
                    loan.spool_charge(),
                    loan.output_charge(),
                    loan.work_steps(),
                ];
                assert_eq!(repeated[2], repeat_before[2]);
                for i in [0, 1] {
                    assert!(repeated[i] >= repeat_before[i]);
                }
                if axis == 3 {
                    assert_eq!(repeated, repeat_before);
                }
                if axis != 3 {
                    // Census work repeats, while fewer vector reservations may
                    // fit after the first refusal consumed cumulative room.
                    assert!(repeated[3] > repeat_before[3]);
                    assert!(
                        repeated[3] - repeat_before[3] <= repeat_before[3] - baseline_before[3]
                    );
                }
                eprintln!(
                    "navigation refused axis={axis} first={repeat_before:?} repeated={repeated:?}"
                );
                accepted = repeated;
                Err("late navigation continuation sentinel")
            },
        );
        assert!(delivered, "axis {axis}: {result:?}");
        assert_eq!(
            result.unwrap(),
            Err("late navigation continuation sentinel")
        );
        assert_eq!(paint_observation(budget.observation()), accepted);
        eprintln!("navigation continuation late axis={axis} actual={accepted:?}");
        let mut redelivered = false;
        assert!(
            navigation_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| redelivered =
                true)
            .is_err()
        );
        assert!(!redelivered);
        let after = paint_observation(budget.observation());
        for i in [0, 1, 3] {
            assert!(after[i] >= accepted[i]);
        }
    }
}

#[test]
fn book_v2_column_navigation_continuation_output_cap_uses_actual_peak_without_reviving_credits() {
    let baseline_caps = display_caps();
    let data = description_data();
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &baseline_caps);
    let mut warm = BookV2ColumnPageBudget::new(&baseline_caps, MAXIMUM);
    let mut peak = 0;
    let mut content_output = 0;
    navigation_drive(
        &input,
        &baseline_caps,
        MODE,
        &mut warm,
        4,
        |bundle, _, _| {
            content_output = bundle.relations().output_charge();
            peak = bundle
                .encoded_prefix()
                .output_charge()
                .max(bundle.marked().content().source().output_charge())
                .max(content_output);
        },
    )
    .unwrap();
    for short in [false, true] {
        let cap = peak - u64::from(short);
        let caps = changed_caps(&baseline_caps, |c| c.max_output_bytes = cap);
        let root = Root::new();
        let input = display_input(&root, data.clone(), None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut delivered = false;
        let result = navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, loan, observed| {
                delivered = true;
                assert_eq!(paint_observation(observed)[2], content_output);
                for _ in 0..2 {
                    let structure = loan.build().unwrap();
                    assert_eq!(structure.fingerprint(), bundle.navigation().fingerprint());
                    assert_eq!(loan.output_charge(), content_output);
                    assert_eq!(structure.output_charge(), content_output);
                }
            },
        );
        if short {
            assert!(result.is_err());
            assert!(!delivered);
        } else {
            result.unwrap();
            assert!(delivered);
            assert_eq!(budget.observation().output_charge(), content_output);
        }
        assert!(budget.observation().output_charge() <= cap);
    }
}
