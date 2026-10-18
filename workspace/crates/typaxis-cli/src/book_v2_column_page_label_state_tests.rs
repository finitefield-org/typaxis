//! Synthetic feedback-state controls, not independent layout/PDF evidence.
use super::*;
use typaxis_core::{ResourceLimits, ValidatedResourceLimits};

fn limits(change: impl FnOnce(&mut ResourceLimits)) -> M4EffectiveResourceLimits {
    let mut base = ResourceLimits::default();
    change(&mut base);
    M4EffectiveResourceLimits::defaults_for(&ValidatedResourceLimits::new(base).unwrap())
}
fn state(count: u32, maximum: u64) -> (ColumnPageLabels, BookV2ColumnPageBudget) {
    let limits = limits(|_| {});
    let mut budget = BookV2ColumnPageBudget::new(&limits, maximum);
    let owners: Vec<_> = (1..=count).map(NodeId::new).collect();
    (
        ColumnPageLabels::from_owners(&owners, &limits, &mut budget).unwrap(),
        budget,
    )
}
fn observations(labels: &ColumnPageLabels, target: u32) -> Vec<(NodeId, u32, Option<u32>, bool)> {
    labels
        .values()
        .iter()
        .map(|(owner, candidate)| (*owner, *candidate, Some(target), true))
        .collect()
}
fn observe(
    labels: &mut ColumnPageLabels,
    budget: &mut BookV2ColumnPageBudget,
    target: u32,
    matching: bool,
    d: u8,
    p: u8,
) -> Result<ColumnLabelDecision, E> {
    labels.observe(
        observations(labels, target).into_iter(),
        matching,
        [d; 32],
        [p; 32],
        budget,
    )
}

#[test]
fn book_v2_column_page_labels_empty_accepts_one_actual_state_and_delivers_once() {
    let (mut labels, mut budget) = state(0, 100);
    assert_eq!(
        labels
            .observe([].into_iter(), true, [1; 32], [2; 32], &mut budget)
            .unwrap(),
        ColumnLabelDecision::Ready
    );
    let before = prefix(&budget);
    assert!(matches!(
        labels.observe([].into_iter(), true, [1; 32], [2; 32], &mut budget),
        Err(E::Identity)
    ));
    assert_eq!(prefix(&budget), before);
}

#[test]
fn book_v2_column_page_labels_require_two_equal_complete_matching_states() {
    let (mut labels, mut budget) = state(1, 100);
    assert_eq!(
        observe(&mut labels, &mut budget, 1, true, 1, 2).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(
        observe(&mut labels, &mut budget, 1, true, 3, 2).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(
        observe(&mut labels, &mut budget, 1, true, 3, 4).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(
        observe(&mut labels, &mut budget, 1, true, 3, 4).unwrap(),
        ColumnLabelDecision::Ready
    );
}

#[test]
fn book_v2_column_page_labels_keep_ten_and_hundred_as_candidates_until_confirmation() {
    let (mut labels, mut budget) = state(3, 1000);
    assert_eq!(
        observe(&mut labels, &mut budget, 10, false, 1, 1).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert!(labels.values().iter().all(|(_, value)| *value == 10));
    assert_eq!(
        observe(&mut labels, &mut budget, 100, false, 2, 2).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert!(labels.values().iter().all(|(_, value)| *value == 100));
    assert_eq!(
        observe(&mut labels, &mut budget, 100, true, 3, 3).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(
        observe(&mut labels, &mut budget, 100, true, 3, 3).unwrap(),
        ColumnLabelDecision::Ready
    );
}

#[test]
fn book_v2_column_page_labels_detect_a_repeating_map_with_constant_cycle_state() {
    let (mut labels, mut budget) = state(1, 1000);
    assert_eq!(
        observe(&mut labels, &mut budget, 2, false, 1, 1).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(
        observe(&mut labels, &mut budget, 1, false, 2, 2).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert!(matches!(
        observe(&mut labels, &mut budget, 2, false, 3, 3),
        Err(E::Limit("column Page label cycle"))
    ));
    assert!(!labels.delivered);
}

#[test]
fn book_v2_column_page_labels_preserve_explicit_unplaced_none_without_a_destination() {
    let (mut labels, mut budget) = state(1, 1000);
    let unplaced = [(NodeId::new(1), 1, None, false)];
    assert_eq!(
        labels
            .observe(unplaced.into_iter(), true, [1; 32], [2; 32], &mut budget)
            .unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(labels.values(), &[(NodeId::new(1), 1)]);
    assert_eq!(
        labels
            .observe(unplaced.into_iter(), true, [1; 32], [2; 32], &mut budget)
            .unwrap(),
        ColumnLabelDecision::Ready
    );
    let (mut labels, mut budget) = state(1, 1000);
    assert!(labels
        .observe(
            [(NodeId::new(1), 1, None, true)].into_iter(),
            false,
            [1; 32],
            [2; 32],
            &mut budget
        )
        .is_err());
    assert!(!labels.delivered);
}

#[test]
fn book_v2_column_page_labels_reject_incomplete_or_foreign_observations_before_delivery() {
    let invalid = [
        vec![],
        vec![(NodeId::new(2), 1, Some(1), true)],
        vec![(NodeId::new(1), 2, Some(2), true)],
        vec![(NodeId::new(1), 1, Some(0), true)],
        vec![(NodeId::new(1), 1, Some(2), true)],
    ];
    for rows in invalid {
        let (mut labels, mut budget) = state(1, 1000);
        let before = prefix(&budget);
        assert!(matches!(
            labels.observe(rows.into_iter(), true, [1; 32], [2; 32], &mut budget),
            Err(E::Identity)
        ));
        assert!(!labels.delivered);
        assert_eq!(prefix(&budget)[..2], before[..2]);
        assert!(prefix(&budget)[2] > before[2]);
    }
}

#[test]
fn book_v2_column_page_labels_prepay_both_requested_buffers_with_exact_boundaries() {
    for count in [1u32, 10, 100] {
        let (labels, budget) = state(count, 10000);
        let wanted = [
            u64::from(count) * 2,
            u64::from(count) * 2 * std::mem::size_of::<(NodeId, u32)>() as u64,
            u64::from(count) * 2 + 3,
        ];
        assert_eq!(prefix(&budget), wanted);
        assert!(
            labels.values.capacity() >= count as usize && labels.next.capacity() >= count as usize
        );
        let owners: Vec<_> = (1..=count).map(NodeId::new).collect();
        for axis in 0..3 {
            let exact = limits(|base| {
                base.max_fragments = wanted[0];
                base.max_spool_bytes = wanted[1];
            });
            let mut budget = BookV2ColumnPageBudget::new(&exact, wanted[2]);
            assert!(ColumnPageLabels::from_owners(&owners, &exact, &mut budget).is_ok());
            assert_eq!(prefix(&budget), wanted);
            let short = limits(|base| {
                base.max_fragments = wanted[0] - u64::from(axis == 0);
                base.max_spool_bytes = wanted[1] - u64::from(axis == 1);
            });
            let mut budget = BookV2ColumnPageBudget::new(&short, wanted[2] - u64::from(axis == 2));
            assert!(ColumnPageLabels::from_owners(&owners, &short, &mut budget).is_err());
            let accepted = prefix(&budget);
            match axis {
                0 => assert_eq!(accepted[0], u64::from(count)),
                1 => assert_eq!(&accepted[..2], &[wanted[0], wanted[1] / 2]),
                _ => assert_eq!(accepted, [wanted[0], wanted[1], wanted[2] - 1]),
            }
        }
    }
}

#[test]
fn book_v2_column_page_labels_retain_late_work_and_reject_a_reset_budget() {
    let count = 3u32;
    let constructor = u64::from(count) * 2 + 3;
    let observed = u64::from(count) * 2 + 4;
    let (mut labels, mut budget) = state(count, constructor + observed);
    assert_eq!(
        observe(&mut labels, &mut budget, 10, false, 1, 1).unwrap(),
        ColumnLabelDecision::Retry
    );
    assert_eq!(prefix(&budget)[2], constructor + observed);
    let (mut labels, mut budget) = state(count, constructor + observed - 1);
    let before = prefix(&budget);
    assert!(observe(&mut labels, &mut budget, 10, false, 1, 1).is_err());
    assert_eq!(&prefix(&budget)[..2], &before[..2]);
    assert_eq!(prefix(&budget)[2], constructor + observed - 1);
    assert!(!labels.delivered);
    let caps = limits(|_| {});
    let mut reset = BookV2ColumnPageBudget::new(&caps, constructor + observed - 1);
    assert!(matches!(
        observe(&mut labels, &mut reset, 10, false, 1, 1),
        Err(E::Identity)
    ));
    assert_eq!(prefix(&reset), [0; 3]);
}

#[test]
fn book_v2_column_page_labels_lend_and_rotate_the_two_paid_buffers_without_copying() {
    let (mut labels, mut budget) = state(1, 1000);
    let pointers = [labels.values.as_ptr(), labels.next.as_ptr()];
    let capacities = [labels.values.capacity(), labels.next.capacity()];
    let before = prefix(&budget);
    let rejected: Result<(), E> = labels.with_candidate_values(|values, state| {
        assert_eq!(values.as_ptr(), pointers[0]);
        assert!(state.values.is_empty());
        assert!(matches!(
            state.with_candidate_values(|_, _| Ok(())),
            Err(E::Identity)
        ));
        Err(E::Limit("candidate source sentinel"))
    });
    assert!(rejected.is_err());
    assert_eq!([labels.values.as_ptr(), labels.next.as_ptr()], pointers);
    assert_eq!(prefix(&budget), before);
    let decision = labels
        .with_candidate_values(|values, state| {
            let rows = [(NodeId::new(1), 1, Some(10), true)];
            let decision = state.observe_total(
                values,
                rows.into_iter(),
                false,
                [1; 32],
                [2; 32],
                &mut budget.inner.observation,
            )?;
            assert!(matches!(
                state.observe_total(
                    values,
                    rows.into_iter(),
                    false,
                    [1; 32],
                    [2; 32],
                    &mut budget.inner.observation
                ),
                Err(E::Identity)
            ));
            Ok(decision)
        })
        .unwrap();
    assert_eq!(decision, ColumnLabelDecision::Retry);
    assert_eq!(
        [labels.values.as_ptr(), labels.next.as_ptr()],
        [pointers[1], pointers[0]]
    );
    assert_eq!(
        [labels.values.capacity(), labels.next.capacity()],
        [capacities[1], capacities[0]]
    );
    assert_eq!(labels.values(), &[(NodeId::new(1), 10)]);
    assert_eq!(&prefix(&budget)[..2], &before[..2]);
}
