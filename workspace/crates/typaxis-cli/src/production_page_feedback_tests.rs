#[test]
fn production_page_feedback_compares_actual_selections_and_accumulates_records() {
    let value = production_body_fixture(3_000_000);
    for ceiling in [82, 81, 41] {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: ceiling,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let once = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(once.record_charge(), 40);
            let result = typaxis_pagination::paginate_stable_production_body(lines, blocks, limits);
            if ceiling == 82 {
                let stable = result.unwrap();
                assert_eq!(stable.passes().len(), 2);
                assert_eq!(stable.passes()[0].index(), 0);
                assert_eq!(stable.passes()[1].index(), 1);
                assert_eq!(
                    stable.passes()[0].selected_fingerprint(),
                    once.fingerprint()
                );
                assert_eq!(
                    stable.passes()[1].selected_fingerprint(),
                    once.fingerprint()
                );
                assert_eq!(stable.passes()[0].cumulative_record_charge(), 41);
                assert_eq!(stable.passes()[1].cumulative_record_charge(), 82);
                assert_eq!(stable.selected().record_charge(), 82);
                assert_eq!(stable.selected().pages().len(), once.pages().len());
                assert_eq!(stable.passes()[1].page_count() as usize, once.pages().len());
                assert!(stable.selected().math_terminals().is_none());
                stable.selected().verify(lines, blocks, limits).unwrap();
            } else {
                assert_eq!(
                    result.err().unwrap().kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit
                );
            }
        });
    }
}

#[test]
fn production_page_feedback_requires_two_passes_even_for_identical_static_input() {
    let value = production_body_fixture(3_000_000);
    for limit in [1, 2] {
        let cfg = config_with_limits(ResourceLimits {
            max_layout_passes: limit,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let result = typaxis_pagination::paginate_stable_production_body(lines, blocks, limits);
            if limit == 1 {
                assert_eq!(
                    result.err().unwrap().kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::PagePassLimit
                );
            } else {
                assert_eq!(result.unwrap().passes().len(), 2);
            }
        });
    }
}

#[test]
fn production_page_feedback_proof_rejects_another_preparation_and_survives_terminals() {
    let value = production_body_fixture(3_000_000);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            let once = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let stable =
                typaxis_pagination::paginate_stable_production_body(lines, blocks, limits).unwrap();
            let (selected, proof) = stable.into_parts();
            proof.verify(&selected, limits).unwrap();
            // Matching placement bytes alone cannot omit the prior pass work.
            assert!(proof.verify(&once, limits).is_err());
            with_production_body_inputs(
                &value,
                &config(),
                |other_lines, other_blocks, other_limits| {
                    let other = typaxis_pagination::paginate_stable_production_body(
                        other_lines,
                        other_blocks,
                        other_limits,
                    )
                    .unwrap();
                    assert!(proof.verify(other.selected(), other_limits).is_err());
                },
            );
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            assert!(selected.math_terminals().is_some());
            proof.verify(&selected, limits).unwrap();
        },
    );
}
