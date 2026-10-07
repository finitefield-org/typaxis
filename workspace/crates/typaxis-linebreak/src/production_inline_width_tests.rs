use super::*;

#[test]
fn refined_widths_match_all_legal_subdivisions_and_exact_budgets() {
    let p = paragraph(
        4,
        (0..4)
            .map(|i| ProductionTextClusterRange {
                start_unit: i,
                end_unit: i + 1,
            })
            .collect(),
    );
    for pattern in 0..81u32 {
        let mut digits = pattern;
        let sizes: Vec<_> = (0..4)
            .map(|_| {
                let size = positive([5, 10, 15][(digits % 3) as usize]);
                digits /= 3;
                size
            })
            .collect();
        for retained in 0..8u32 {
            let ends: Vec<_> = (1..=4)
                .filter(|end| *end == 4 || retained & (1 << (end - 1)) != 0)
                .collect();
            let binding =
                ProductionInlineSourceWidths::with_retained_line_ends(&p, &sizes, &ends).unwrap();
            let mut budget = ProductionLineBreakBudget::new(1000, 100);
            let selected =
                break_production_inline_with_source_widths(&binding, positive(10), &mut budget)
                    .unwrap();
            let mut best = i64::MAX;
            for cuts in 0..8u32 {
                if cuts & retained != retained {
                    continue;
                }
                let mut start = 0;
                let mut cost = 0;
                let mut feasible = true;
                for end in 1..=4 {
                    if end != 4 && cuts & (1 << (end - 1)) == 0 {
                        continue;
                    }
                    let available = sizes[start].get().raw();
                    let used = (end - start) as i64 * 5;
                    if used > available {
                        feasible = false;
                        break;
                    }
                    let ratio = (available - used) * 1000 / available;
                    let penalty = if end == 4 {
                        0
                    } else {
                        i64::from(p.boundaries[end - 1].penalty())
                    };
                    let badness = ratio * ratio * ratio / 1_000_000;
                    cost += (badness
                        + if penalty >= 0 {
                            penalty * penalty
                        } else {
                            -penalty * penalty
                        })
                    .max(0);
                    start = end;
                }
                if feasible {
                    best = best.min(cost);
                }
            }
            assert_eq!(
                selected.lines().last().unwrap().line().break_demerits(),
                best
            );
            for end in &ends {
                assert!(selected
                    .lines()
                    .iter()
                    .any(|line| line.line().end_unit() == *end));
            }
            assert!(selected
                .canonical_jcs()
                .contains(PRODUCTION_REFINED_WIDTH_BREAK_ALGORITHM));
            let work = 1000 - budget.remaining_steps();
            let lines = 100 - budget.remaining_lines();
            let run = |work, lines| {
                break_production_inline_with_source_widths(
                    &binding,
                    positive(10),
                    &mut ProductionLineBreakBudget::new(work, lines),
                )
            };
            assert_eq!(
                run(work, lines).unwrap().fingerprint(),
                selected.fingerprint()
            );
            assert!(matches!(
                run(work - 1, lines),
                Err(AtomicVectorInlineError::CandidateLimit)
            ));
            assert!(matches!(
                run(work, lines - 1),
                Err(AtomicVectorInlineError::SelectionLimit)
            ));
        }
    }
}

#[test]
fn refined_widths_reject_invalid_boundaries_and_keep_cluster_atomicity() {
    let p = paragraph(
        3,
        vec![
            ProductionTextClusterRange {
                start_unit: 0,
                end_unit: 2,
            },
            ProductionTextClusterRange {
                start_unit: 2,
                end_unit: 3,
            },
        ],
    );
    let sizes = [positive(100); 3];
    let run = |ends: &[u32]| {
        let binding =
            ProductionInlineSourceWidths::with_retained_line_ends(&p, &sizes, ends).unwrap();
        break_production_inline_with_source_widths(
            &binding,
            positive(10),
            &mut ProductionLineBreakBudget::new(100, 10),
        )
    };
    for ends in [&[][..], &[0, 3], &[1, 3], &[2, 2, 3], &[3, 2], &[4], &[2]] {
        assert!(
            matches!(run(ends), Err(AtomicVectorInlineError::InvalidBinding)),
            "{ends:?}"
        );
    }
    let refined = run(&[2, 3]).unwrap();
    assert_eq!(refined.lines().len(), 2);
    assert_eq!(refined.lines()[0].line().break_kind(), BreakKind::Allowed);
    assert_ne!(refined.fingerprint(), run(&[3]).unwrap().fingerprint());
    let empty = ProductionInlineParagraph::empty(NodeId::new(1));
    let sizes = [positive(7)];
    for ends in [&[][..], &[0, 0], &[1]] {
        let binding =
            ProductionInlineSourceWidths::with_retained_line_ends(&empty, &sizes, ends).unwrap();
        assert!(break_production_inline_with_source_widths(
            &binding,
            positive(10),
            &mut ProductionLineBreakBudget::new(100, 10)
        )
        .is_err());
    }
    let binding =
        ProductionInlineSourceWidths::with_retained_line_ends(&empty, &sizes, &[0]).unwrap();
    let result = break_production_inline_with_source_widths(
        &binding,
        positive(10),
        &mut ProductionLineBreakBudget::new(3, 1),
    )
    .unwrap();
    assert_eq!(result.lines().len(), 1);
}
fn positive(raw: i64) -> PositiveLength {
    PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
}
fn paragraph(count: u32, clusters: Vec<ProductionTextClusterRange>) -> ProductionInlineParagraph {
    ProductionInlineParagraph::itemize(
        NodeId::new(1),
        (0..count)
            .map(|_| {
                AtomicVectorInlineLogicalUnit::Text(AtomicVectorTextUnit::new(
                    '日',
                    NonNegativeLength::new(Length::from_raw(5).unwrap()).unwrap(),
                    NonNegativeLength::ZERO,
                    NonNegativeLength::ZERO,
                ))
            })
            .collect(),
        clusters,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap()
}
#[test]
fn source_widths_match_exhaustive_partition_costs_and_preserve_source_ranges() {
    let p = paragraph(
        6,
        (0..6)
            .map(|i| ProductionTextClusterRange {
                start_unit: i,
                end_unit: i + 1,
            })
            .collect(),
    );
    for pattern in 0..729u32 {
        let mut digits = pattern;
        let sizes: Vec<_> = (0..6)
            .map(|_| {
                let size = positive([5, 10, 15][(digits % 3) as usize]);
                digits /= 3;
                size
            })
            .collect();
        let widths = ProductionInlineSourceWidths::new(&p, &sizes).unwrap();
        assert!(std::ptr::eq(widths.source(), &p));
        let mut budget = ProductionLineBreakBudget::new(1000, 100);
        let selected =
            break_production_inline_with_source_widths(&widths, positive(10), &mut budget).unwrap();
        // Enumerate every complete partition, independently of the DP state.
        let mut best = i64::MAX;
        for cuts in 0..32u32 {
            let mut start = 0usize;
            let mut total = 0i64;
            let mut feasible = true;
            for end in 1..=6 {
                if end != 6 && cuts & (1 << (end - 1)) == 0 {
                    continue;
                }
                let available = sizes[start].get().raw();
                let used = (end - start) as i64 * 5;
                if used > available {
                    feasible = false;
                    break;
                }
                let ratio = (available - used) * 1000 / available;
                let penalty = if end == 6 {
                    0
                } else {
                    i64::from(p.boundaries[end - 1].penalty())
                };
                let badness = ratio * ratio * ratio / 1_000_000;
                total += (badness
                    + if penalty >= 0 {
                        penalty * penalty
                    } else {
                        -penalty * penalty
                    })
                .max(0);
                start = end;
            }
            if feasible {
                best = best.min(total);
            }
        }
        assert_eq!(
            selected.lines().last().unwrap().line().break_demerits(),
            best,
            "{pattern}"
        );
        let mut start = 0;
        for line in selected.lines() {
            assert_eq!(line.line().start_unit(), start);
            assert_eq!(line.inline_size(), sizes[start as usize]);
            assert!(line.required_inline_size().get() <= line.inline_size().get());
            start = line.line().end_unit();
        }
        assert_eq!(start, 6);
        let work = 1000 - budget.remaining_steps();
        let lines = 100 - budget.remaining_lines();
        let exact = break_production_inline_with_source_widths(
            &widths,
            positive(10),
            &mut ProductionLineBreakBudget::new(work, lines),
        )
        .unwrap();
        assert_eq!(exact.fingerprint(), selected.fingerprint());
        assert!(matches!(
            break_production_inline_with_source_widths(
                &widths,
                positive(10),
                &mut ProductionLineBreakBudget::new(work - 1, lines)
            ),
            Err(AtomicVectorInlineError::CandidateLimit)
        ));
        assert!(matches!(
            break_production_inline_with_source_widths(
                &widths,
                positive(10),
                &mut ProductionLineBreakBudget::new(work, lines - 1)
            ),
            Err(AtomicVectorInlineError::SelectionLimit)
        ));
    }
}
#[test]
fn source_widths_do_not_split_or_skip_a_cluster_to_reach_a_wider_region() {
    let p = paragraph(
        3,
        vec![
            ProductionTextClusterRange {
                start_unit: 0,
                end_unit: 2,
            },
            ProductionTextClusterRange {
                start_unit: 2,
                end_unit: 3,
            },
        ],
    );
    let sizes = [positive(10), positive(1), positive(5)];
    let widths = ProductionInlineSourceWidths::new(&p, &sizes).unwrap();
    let result = break_production_inline_with_source_widths(
        &widths,
        positive(10),
        &mut ProductionLineBreakBudget::new(100, 10),
    )
    .unwrap();
    assert_eq!(
        result
            .lines()
            .iter()
            .map(|l| (
                l.line().start_unit(),
                l.line().end_unit(),
                l.inline_size().get().raw()
            ))
            .collect::<Vec<_>>(),
        [(0, 2, 10), (2, 3, 5)]
    );
    let changed = [positive(10), positive(99), positive(5)];
    let other = break_production_inline_with_source_widths(
        &ProductionInlineSourceWidths::new(&p, &changed).unwrap(),
        positive(10),
        &mut ProductionLineBreakBudget::new(100, 10),
    )
    .unwrap();
    assert_ne!(
        other.fingerprint(),
        result.fingerprint(),
        "unselected widths must remain bound"
    );
    let narrow = [positive(9), positive(100), positive(100)];
    assert!(matches!(
        break_production_inline_with_source_widths(
            &ProductionInlineSourceWidths::new(&p, &narrow).unwrap(),
            positive(10),
            &mut ProductionLineBreakBudget::new(100, 10)
        ),
        Err(AtomicVectorInlineError::NoFeasibleLine)
    ));
    assert!(ProductionInlineSourceWidths::new(&p, &sizes[..2]).is_err());
}
#[test]
fn source_widths_keep_empty_lines_and_shared_budget_without_fixed_receipt_aliasing() {
    let p = ProductionInlineParagraph::empty(NodeId::new(1));
    let sizes = [positive(7)];
    assert!(ProductionInlineSourceWidths::new(&p, &[]).is_err());
    let widths = ProductionInlineSourceWidths::new(&p, &sizes).unwrap();
    let mut budget = ProductionLineBreakBudget::new(2, 1);
    let result =
        break_production_inline_with_source_widths(&widths, positive(10), &mut budget).unwrap();
    assert_eq!(result.lines().len(), 1);
    assert_eq!(result.lines()[0].inline_size(), positive(7));
    assert_eq!((budget.remaining_steps(), budget.remaining_lines()), (0, 0));
    assert!(
        break_production_inline_with_source_widths(&widths, positive(10), &mut budget).is_err()
    );
    let fixed = break_production_inline(
        &p,
        positive(7),
        positive(10),
        &mut ProductionLineBreakBudget::new(1, 1),
    )
    .unwrap();
    assert_ne!(result.fingerprint(), fixed.fingerprint());
    assert!(result
        .canonical_jcs()
        .contains(PRODUCTION_SOURCE_WIDTH_BREAK_ALGORITHM));
}
