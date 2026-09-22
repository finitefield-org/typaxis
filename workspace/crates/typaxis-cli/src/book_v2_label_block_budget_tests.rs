use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_layout::book_v2::{
    prepare_book_v2_vector_blocks, prepare_book_v2_vector_blocks_counted,
    with_budgeted_book_v2_body_lines_with_source_widths, BookV2BodyLineBudget,
    BookV2VectorBlockError,
};
use typaxis_shaping::book_v2::{shape_book_v2_equation_numbers_counted, BookV2EquationNumberError};

#[test]
fn book_v2_label_block_budget_retains_records_on_failure() {
    check(None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_label_block_budget_retains_original_harano_records() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&bytes));
}

fn check(font: Option<&[u8]>) {
    for mode in [
        "empty",
        "single",
        "multiple",
        "missing-style",
        "spool",
        "overlap",
    ] {
        let root = Root::new();
        let limits = if mode == "spool" {
            let mut caps = limits().base().get().clone();
            caps.max_spool_bytes = 4096;
            M4EffectiveResourceLimits::new(
                ValidatedResourceLimits::new(caps).unwrap(),
                M4ResourceLimits {
                    max_font_subset_bytes: 4096,
                    ..M4ResourceLimits::default()
                },
            )
            .unwrap()
        } else {
            limits()
        };
        let mut data = match mode {
            "empty" => source_data(std::str::from_utf8(VECTOR_SOURCE).unwrap()),
            "missing-style" => vector_data(),
            "overlap" => blocks::block_data("end", true),
            _ => blocks::block_data("start", true),
        };
        if mode == "multiple" {
            let blocks = data["document"]["blocks"][0]["blocks"]
                .as_array_mut()
                .unwrap();
            let mut number = blocks[2].clone();
            number["node_id"] = 10.into();
            number["equation_number"]["node_id"] = 11.into();
            blocks.push(number);
        }
        data["page_masters"]["masters"][0]["body"] =
            json!({"x":500000,"y":500000,"width":10000000,"height":20000000});
        if let Some(font) = font {
            data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
        }
        let body = body_with_source(&root, data, VECTOR_SOURCE, &limits);
        if let Some(font) = font {
            fs::write(root.0.join("body.bin"), font).unwrap();
        }
        fs::write(root.0.join("vector.svg"), VECTOR).unwrap();
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config_with_extension(
                limits.base().get().clone(),
                limits.extension().get().clone(),
            ),
            &limits,
        )
        .unwrap();
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let maximum = 100_000_000;
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
            &flow, &mut 0, maximum, 0, 0,
        )
        .unwrap();
        assert!(!plan.requires_width_reflow());
        let mut allowance =
            BookV2BodyLineBudget::new(maximum, limits.base().get().max_line_reshape_passes);
        with_budgeted_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            plan.measurement_body(),
            None,
            &mut allowance,
            Some(&plan),
            None,
            |stable| {
                let lines = stable.lines();
                let shaped = lines.prepared().shaped();
                let initial = plan.record_charge() + stable.footnotes().record_charge();
                let mut records = u64::MAX;
                let numbers =
                    shape_book_v2_equation_numbers_counted(shaped, &limits, initial, &mut records);
                if mode == "missing-style" || mode == "spool" {
                    let error = numbers.err().unwrap();
                    assert_eq!(
                        error.kind,
                        if mode == "spool" {
                            E::SpoolLimit
                        } else {
                            E::MissingTextStyle
                        }
                    );
                    assert_eq!(
                        error,
                        shape_book_v2_equation_numbers(shaped, &limits, initial)
                            .err()
                            .unwrap()
                    );
                    assert!(records > initial);
                    if mode == "spool" {
                        assert!(records > initial + 2);
                    }
                    driver_failure(&input, &limits, "equation labels", records, Some(error));
                    let before = records;
                    assert!(shape_book_v2_equation_numbers_counted(
                        shaped,
                        &limits,
                        before,
                        &mut records
                    )
                    .is_err());
                    assert!(records >= before);
                    return;
                }
                let numbers = numbers.unwrap();
                let number_records = records;
                if let Some(numbers) = &numbers {
                    assert_eq!(records, numbers.record_charge());
                    assert_eq!(
                        numbers.fingerprint(),
                        shape_book_v2_equation_numbers(shaped, &limits, initial)
                            .unwrap()
                            .unwrap()
                            .fingerprint()
                    );
                    let extra = records - initial;
                    let cap = limits.base().get().max_fragments;
                    let mut partial = false;
                    for remaining in 0..extra {
                        let prior = cap - remaining;
                        let error = shape_book_v2_equation_numbers_counted(
                            shaped,
                            &limits,
                            prior,
                            &mut records,
                        )
                        .err()
                        .unwrap();
                        assert_eq!(
                            error,
                            shape_book_v2_equation_numbers(shaped, &limits, prior)
                                .err()
                                .unwrap()
                        );
                        assert!(records >= prior && records <= cap);
                        partial |= records > prior;
                    }
                    assert!(partial);
                    let exact = shape_book_v2_equation_numbers_counted(
                        shaped,
                        &limits,
                        cap - extra,
                        &mut records,
                    )
                    .unwrap()
                    .unwrap();
                    assert_eq!(records, cap);
                    assert_eq!(exact.fingerprint(), numbers.fingerprint());
                } else {
                    assert_eq!(number_records, initial);
                }
                let result = prepare_book_v2_vector_blocks_counted(
                    lines,
                    numbers.as_ref(),
                    &limits,
                    initial,
                    &mut records,
                );
                if mode == "overlap" {
                    let error = result.err().unwrap();
                    assert!(matches!(error, BookV2VectorBlockError::Geometry(_)));
                    assert_eq!(
                        format!("{error:?}"),
                        format!(
                            "{:?}",
                            prepare_book_v2_vector_blocks(
                                lines,
                                numbers.as_ref(),
                                &limits,
                                initial
                            )
                            .err()
                            .unwrap()
                        )
                    );
                    assert!(records > number_records);
                    driver_failure(&input, &limits, "block layout", records, None);
                    return;
                }
                let Some(blocks) = result.unwrap() else {
                    assert_eq!(mode, "empty");
                    assert_eq!(records, initial.max(lines.output_records()));
                    return;
                };
                assert_eq!(records, blocks.record_charge());
                assert_eq!(
                    blocks.fingerprint(),
                    prepare_book_v2_vector_blocks(lines, numbers.as_ref(), &limits, initial)
                        .unwrap()
                        .unwrap()
                        .fingerprint()
                );
                let cap = limits.base().get().max_fragments;
                let extra = records - initial;
                let prior = cap - extra;
                let exact = prepare_book_v2_vector_blocks_counted(
                    lines,
                    numbers.as_ref(),
                    &limits,
                    prior,
                    &mut records,
                )
                .unwrap()
                .unwrap();
                assert_eq!(records, cap);
                assert_eq!(exact.fingerprint(), blocks.fingerprint());
                assert!(matches!(
                    prepare_book_v2_vector_blocks_counted(
                        lines,
                        numbers.as_ref(),
                        &limits,
                        prior + 1,
                        &mut records
                    ),
                    Err(BookV2VectorBlockError::OutputLimit)
                ));
                assert_eq!(
                    records,
                    prior + 1 + numbers.as_ref().map_or(0, |n| n.retained_records())
                );
                // Input histories survive validation errors before block allocation.
                let mut changed = limits.base().get().clone();
                changed.max_pages -= 1;
                let changed = M4EffectiveResourceLimits::new(
                    ValidatedResourceLimits::new(changed).unwrap(),
                    limits.extension().get().clone(),
                )
                .unwrap();
                assert!(matches!(
                    prepare_book_v2_vector_blocks_counted(
                        lines,
                        numbers.as_ref(),
                        &changed,
                        initial,
                        &mut records
                    ),
                    Err(BookV2VectorBlockError::ReceiptMismatch(_))
                ));
                assert_eq!(
                    records,
                    initial.max(lines.output_records()).max(number_records)
                );
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
        if matches!(mode, "missing-style" | "spool" | "overlap") {
            use crate::book_v2_resources::converged_pdf::header_catalog_driver::{
                with_header_catalog, HeaderCatalogBudget,
            };
            let seed = typaxis_layout::book_v2::prepare_book_v2_body_line_variant_seed(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                JapaneseLineBreakMode::Normal,
                plan.measurement_body(),
                maximum,
                0,
                None,
                limits.base().get().max_line_reshape_passes,
                Some(&plan),
                None,
            )
            .unwrap();
            let mut budget = HeaderCatalogBudget::default();
            let result: Result<(), _> =
                with_header_catalog(&seed, &[], &limits, maximum, &mut budget, |_, _| {
                    panic!("failed header preparation reached catalog consumer")
                });
            let BookV2ConvergenceError::Stage { stage, source } = result.unwrap_err() else {
                panic!("expected header constructor failure")
            };
            if mode == "overlap" {
                assert_eq!(stage, "header block layout");
                assert!(matches!(
                    source.downcast_ref::<BookV2VectorBlockError>().unwrap(),
                    BookV2VectorBlockError::Geometry(_)
                ));
            } else {
                assert_eq!(stage, "header equation labels");
                assert_eq!(
                    source
                        .downcast_ref::<BookV2EquationNumberError>()
                        .unwrap()
                        .kind,
                    if mode == "spool" {
                        E::SpoolLimit
                    } else {
                        E::MissingTextStyle
                    }
                );
            }
            assert!(budget.records > seed.record_charge());
            assert!(budget.work > 0);
            assert_eq!((budget.line_passes, budget.page_passes), (0, 0));
            let retained = budget;
            assert!(
                with_header_catalog(&seed, &[], &limits, maximum, &mut budget, |_, _| Ok(()))
                    .is_err()
            );
            assert!(budget.records >= retained.records && budget.work >= retained.work);
        }
    }
}

fn driver_failure(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    expected_stage: &str,
    records: u64,
    number: Option<BookV2EquationNumberError>,
) {
    let mut budget = BookV2PdfConvergenceBudget::new(limits, 100_000_000);
    let error = with_budgeted_book_v2_pdf(
        input,
        limits,
        JapaneseLineBreakMode::Normal,
        &mut budget,
        |_, _| panic!("failed constructor reached PDF"),
    )
    .unwrap_err();
    let BookV2ConvergenceError::Stage { stage, source } = error else {
        panic!("{error:?}")
    };
    assert_eq!(stage, expected_stage);
    if let Some(number) = number {
        assert_eq!(
            *source.downcast_ref::<BookV2EquationNumberError>().unwrap(),
            number
        );
    } else {
        assert!(matches!(
            source.downcast_ref::<BookV2VectorBlockError>().unwrap(),
            BookV2VectorBlockError::Geometry(_)
        ));
    }
    let retained = budget.observation();
    assert_eq!(retained.record_charge(), records);
    assert_eq!(retained.output_charge(), 0);
    assert_eq!(retained.page_passes(), 0);
    assert_eq!(retained.candidate_passes(), 0);
    assert!(with_budgeted_book_v2_pdf(
        input,
        limits,
        JapaneseLineBreakMode::Normal,
        &mut budget,
        |_, _| ()
    )
    .is_err());
    assert!(budget.observation().record_charge() >= retained.record_charge());
    assert!(budget.observation().work_steps() >= retained.work_steps());
}
