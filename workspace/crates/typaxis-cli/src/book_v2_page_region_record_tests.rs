use super::*;
use typaxis_layout::book_v2::{
    prepare_book_v2_page_region_inlines_counted as counted_inlines,
    with_budgeted_book_v2_page_region_lines as run, BookV2PageRegionLineBudget as Budget,
};
use typaxis_shaping::book_v2::shape_book_v2_page_region_text_counted as counted_shape;

#[test]
fn book_v2_page_region_records_preserve_all_preparation_boundaries() {
    check(None, "Result");
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_page_region_records_preserve_original_harano_boundaries() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font), "本文");
}
fn check(font: Option<&[u8]>, word: &str) {
    for mode in ["success", "indent", "height", "missing-glyph"] {
        let root = Root::new();
        let limits = limits();
        let text = if mode == "missing-glyph" {
            format!("{word}\u{10ffff}")
        } else {
            word.into()
        };
        let mut data = layout_data(
            &text,
            if mode == "indent" {
                4 * 65536
            } else {
                200 * 65536
            },
            if mode == "height" { 1 } else { 95 * 65536 },
            "start",
            true,
        );
        if mode == "missing-glyph" {
            let blocks = &mut data["page_masters"]["masters"][0]["footer_content"]["blocks"];
            blocks[0]["children"][0]["text_span"]["end_byte"] = word.len().into();
            blocks[1]["children"][0]["text_span"]["start_byte"] = word.len().into();
        }
        let input = if let Some(font) = font {
            vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let selected =
            select_book_v2_page_master(input.body().styled(), 0, None, &mut 0, 1_000_000).unwrap();
        let flow = region_flow(selected, Kind::Footer, &nav, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let mut records = u64::MAX;
        let shape = counted_shape(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            None,
            &mut records,
        );
        let call = |budget: &mut Budget, prior| {
            run(
                &policy,
                &flow,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                selected,
                budget,
                prior,
                |stable| {
                    if prior == 0 {
                        let mut records = 0;
                        let contexts = stable
                            .lines()
                            .selected_line_contexts_counted(&mut records)
                            .unwrap();
                        assert_eq!(records, contexts.record_charge());
                        assert_eq!(
                            contexts.source_fingerprint(),
                            stable
                                .lines()
                                .selected_line_contexts()
                                .unwrap()
                                .source_fingerprint()
                        );
                    }
                    (
                        stable.lines().fingerprint(),
                        stable.lines().output_records(),
                    )
                },
            )
        };
        if mode == "missing-glyph" {
            assert_eq!(
                shape.err().unwrap(),
                shape_region(&policy, &flow, input.resources(), &limits, EPOCH, None)
                    .err()
                    .unwrap()
            );
            assert_eq!(records, 1 + 2 * word.chars().count() as u64);
            let mut budget = Budget::new(1_000_000, 16);
            for _ in 0..2 {
                assert!(call(&mut budget, 0).is_err());
                assert_eq!(budget.record_charge(), records);
                assert_eq!((budget.candidate_steps(), budget.reshape_passes()), (0, 0));
            }
            continue;
        }
        let shape = shape.unwrap();
        assert_eq!(records, shape.output_records());
        assert_eq!(
            shape.fingerprint(),
            shape_region(&policy, &flow, input.resources(), &limits, EPOCH, None)
                .unwrap()
                .fingerprint()
        );
        let prior = 11;
        let prepared = counted_inlines(
            &flow,
            &shape,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            prior,
            &mut records,
        )
        .unwrap();
        assert_eq!(records, prepared.record_charge());
        assert_eq!(
            prepared.fingerprint(),
            inlines(
                &flow,
                &shape,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                prior
            )
            .unwrap()
            .fingerprint()
        );
        let prepared_records = records;
        let mut zero = Budget::new(0, 16);
        assert!(call(&mut zero, prior).is_err());
        let first_units = prepared.paragraphs()[0].items().unwrap().units().len() as u64;
        let paragraphs = prepared.paragraphs().len() as u64;
        let expected_zero = prepared_records
            + 1
            + paragraphs
            + if mode == "indent" {
                0
            } else {
                paragraphs + first_units
            };
        assert_eq!(zero.record_charge(), expected_zero, "{mode}");
        assert_eq!((zero.candidate_steps(), zero.reshape_passes()), (0, 0));
        if mode != "success" {
            let mut budget = Budget::new(1_000_000, 16);
            assert!(call(&mut budget, prior).is_err());
            assert!(budget.record_charge() >= expected_zero);
            assert_eq!(budget.reshape_passes() > 0, mode == "height");
            let records = budget.record_charge();
            assert!(call(&mut budget, prior).is_err());
            assert_eq!(budget.record_charge(), records);
            continue;
        }
        // Every reservation boundary of the inline constructor, with the same
        // admitted source and original shape. A rejected atomic charge is absent.
        let cap = limits.base().get().max_fragments;
        let inline_extra = prepared_records - prior;
        let mut partial = false;
        for remaining in 0..inline_extra {
            let prefix = cap - remaining;
            let cause = counted_inlines(
                &flow,
                &shape,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                prefix,
                &mut records,
            )
            .err()
            .unwrap();
            let legacy = inlines(
                &flow,
                &shape,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                prefix,
            )
            .err()
            .unwrap();
            assert_eq!(format!("{cause:?}"), format!("{legacy:?}"));
            assert!(records >= prefix && records <= cap);
            partial |= records > prefix;
        }
        assert!(partial);
        assert_eq!(
            counted_inlines(
                &flow,
                &shape,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                cap - inline_extra,
                &mut records
            )
            .unwrap()
            .record_charge(),
            cap
        );
        let cause = counted_inlines(
            &flow,
            &shape,
            input.resources(),
            &limits,
            [0; 32],
            MODE,
            prior,
            &mut records,
        )
        .err()
        .unwrap();
        assert!(matches!(cause, Error::Shape(_)));
        assert_eq!(records, prior);
        let mut full = Budget::new(1_000_000, 16);
        let full_output = call(&mut full, 0).unwrap();
        assert!(full.record_charge() >= full_output.1);
        let required = full.record_charge();
        let mut advanced = false;
        for remaining in 0..required {
            let prefix = cap - remaining;
            let mut limited = Budget::new(1_000_000, 16);
            assert!(
                call(&mut limited, prefix).is_err(),
                "remaining={remaining}, required={required}"
            );
            assert!(limited.record_charge() >= prefix && limited.record_charge() <= cap);
            advanced |= limited.reshape_passes() > 0;
        }
        assert!(advanced, "boundaries must reach a begun rebreak");
        let mut exact = Budget::new(1_000_000, 16);
        let exact_output = call(&mut exact, cap - required).unwrap();
        assert_eq!(exact_output.0, full_output.0);
        assert_eq!(exact.record_charge(), cap);
        let mut short = Budget::new(full.candidate_steps() - 1, 16);
        assert!(call(&mut short, 0).is_err());
        assert!(short.record_charge() > 0);
        let records = short.record_charge();
        assert!(call(&mut short, 0).is_err());
        assert!(short.record_charge() >= records);
    }
}
