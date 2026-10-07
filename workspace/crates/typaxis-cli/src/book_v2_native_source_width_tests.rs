use super::*;
use typaxis_layout::book_v2::{
    with_converged_book_v2_body_lines_with_source_widths, BookV2SourceWidthAssignments,
};
#[test]
fn book_v2_native_source_widths_rebind_atoms_and_keep_shared_computation_through_reshape() {
    let root = Root::new();
    let limits = limits();
    let input = native_input(&root, native_data(), &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let native = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let prepared = prepare_book_v2_inline_items_with_native_context(
        &flow,
        &shape,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        Some(&native),
    )
    .unwrap();
    assert_eq!(prepared.paragraphs().len(), 1);
    let items = prepared.paragraphs()[0].items().unwrap();
    use typaxis_linebreak::ProductionInlineLogicalUnit as U;
    let small = items.units()[..7]
        .iter()
        .map(|unit| {
            let U::Text(t) = unit else {
                panic!("original Result prefix")
            };
            t.advance().get().raw()
        })
        .sum::<i64>();
    let width = |n| PositiveLength::new(Length::from_raw(n).unwrap()).unwrap();
    let mut sizes = vec![width(small * 3); items.units().len()];
    sizes[0] = width(small);
    let profiles = [Some(sizes.as_slice())];
    let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
    let envelopes = [width(small * 3)];
    let direct = typaxis_layout::book_v2::layout_book_v2_source_width_lines_from_flow(
        &prepared,
        &envelopes,
        &assignments,
        1_000_000,
        0,
    )
    .unwrap();
    let prior = limits.base().get().max_fragments - direct.output_records();
    let exact = typaxis_layout::book_v2::layout_book_v2_source_width_lines_from_flow(
        &prepared,
        &envelopes,
        &assignments,
        direct.candidate_steps(),
        prior,
    )
    .unwrap();
    assert_eq!(exact.output_records(), limits.base().get().max_fragments);
    assert_eq!(exact.fingerprint(), direct.fingerprint());
    assert!(
        typaxis_layout::book_v2::layout_book_v2_source_width_lines_from_flow(
            &prepared,
            &envelopes,
            &assignments,
            direct.candidate_steps(),
            prior + 1,
        )
        .is_err()
    );
    let frame = typaxis_core::Rect::new(
        Length::ZERO,
        Length::ZERO,
        width(1000 * 65536),
        width(1000 * 65536),
    );
    let run = |work, passes| {
        with_converged_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            frame,
            work,
            Some(&native),
            passes,
            None,
            Some(&assignments),
            |stable| {
                assert!(stable.passes().last().unwrap().is_stable());
                assert!(std::ptr::eq(
                    stable.lines().prepared().native_math().unwrap(),
                    &native
                ));
                let p = &stable.lines().paragraphs()[0];
                assert_eq!(p.selected().unwrap().lines()[0].line().end_unit(), 7);
                assert_eq!(p.line_inline_size(0), Some(width(small)));
                let mut math = 0;
                for line in p.lines() {
                    for item in line.items() {
                        if let ProductionPlacedInline::BookV2Math(m) = item {
                            assert_eq!(m.owner(), NodeId::new(4));
                            math += 1;
                        }
                    }
                }
                assert_eq!(math, 1);
                (
                    stable.candidate_steps(),
                    stable.passes().len() as u16,
                    stable.lines().fingerprint(),
                )
            },
        )
    };
    let full = run(1_000_000, limits.base().get().max_line_reshape_passes).unwrap();
    assert_eq!(run(full.0, full.1).unwrap(), full);
    assert!(run(full.0 - 1, full.1).is_err());
    assert!(run(full.0, full.1 - 1).is_err());

    use typaxis_layout::book_v2::{
        prepare_book_v2_body_line_variant_seed, with_rebuilt_book_v2_body_line_variant,
    };
    let wide_profiles = [None];
    let wide_assignment = BookV2SourceWidthAssignments::new(&flow, &wide_profiles).unwrap();
    let narrow_seed = prepare_book_v2_body_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        frame,
        1_000_000,
        0,
        Some(&native),
        limits.base().get().max_line_reshape_passes,
        None,
        Some(&assignments),
    )
    .unwrap();
    let wide_seed = prepare_book_v2_body_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        frame,
        1_000_000,
        narrow_seed.record_charge(),
        Some(&native),
        limits.base().get().max_line_reshape_passes,
        None,
        Some(&wide_assignment),
    )
    .unwrap();
    with_rebuilt_book_v2_body_line_variant(
        &narrow_seed,
        1_000_000,
        wide_seed.record_charge(),
        |narrow| {
            with_rebuilt_book_v2_body_line_variant(
                &wide_seed,
                1_000_000,
                narrow.record_charge(),
                |wide| {
                    assert!(
                        narrow.lines().paragraphs()[0].lines().len()
                            > wide.lines().paragraphs()[0].lines().len()
                    );
                    for lines in [narrow.lines(), wide.lines()] {
                        assert!(std::ptr::eq(
                            lines.prepared().native_math().unwrap(),
                            &native
                        ));
                        let math = lines.paragraphs()[0]
                            .lines()
                            .iter()
                            .flat_map(|l| l.items())
                            .filter_map(|i| {
                                if let ProductionPlacedInline::BookV2Math(m) = i {
                                    Some(m.owner())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(math, vec![NodeId::new(4)]);
                    }
                    assert!(!std::ptr::eq(
                        narrow.lines().prepared().shaped(),
                        wide.lines().prepared().shaped()
                    ));
                    eprintln!(
                        "native line variant seeds: narrow={},wide={},rebuild={}/{},records={}",
                        narrow_seed.work_steps(),
                        wide_seed.work_steps(),
                        narrow.work_steps(),
                        wide.work_steps(),
                        wide.record_charge()
                    );
                },
            )
            .unwrap()
        },
    )
    .unwrap();

    use typaxis_layout::book_v2::with_rebuilt_book_v2_body_line_variants;
    with_rebuilt_book_v2_body_line_variants(&[&narrow_seed, &wide_seed], 1_000_000, 0, |set| {
        assert_eq!(set.variants().len(), 2);
        for v in set.variants() {
            assert!(std::ptr::eq(
                v.lines().prepared().native_math().unwrap(),
                &native
            ));
        }
        assert!(
            set.variants()[0].lines().paragraphs()[0].lines().len()
                > set.variants()[1].lines().paragraphs()[0].lines().len()
        );
        eprintln!(
            "native line variant set: work={},records={}",
            set.work_steps(),
            set.record_charge()
        );
    })
    .unwrap();
    let duplicate_native = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let duplicate_seed = prepare_book_v2_body_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        frame,
        1_000_000,
        0,
        Some(&duplicate_native),
        limits.base().get().max_line_reshape_passes,
        None,
        Some(&assignments),
    )
    .unwrap();
    assert_eq!(duplicate_seed.fingerprint(), narrow_seed.fingerprint());
    assert!(with_rebuilt_book_v2_body_line_variants(
        &[&narrow_seed, &duplicate_seed],
        1_000_000,
        0,
        |_| ()
    )
    .is_err());
}
