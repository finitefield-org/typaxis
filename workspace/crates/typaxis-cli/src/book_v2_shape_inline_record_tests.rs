use super::*;
use typaxis_layout::book_v2::{
    prepare_book_v2_inline_items_with_native_context,
    prepare_book_v2_inline_items_with_native_context_counted,
};
use typaxis_shaping::book_v2::shape_book_v2_authored_text_counted;

#[test]
fn book_v2_shape_inline_records_preserve_partial_source_preparation() {
    check(None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_shape_inline_records_preserve_original_harano_preparation() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font));
}
fn check(font: Option<&[u8]>) {
    for mode in [
        "success",
        "missing-glyph",
        "figure-width",
        "shape-limit",
        "shape-exact",
    ] {
        let root = Root::new();
        let word = if font.is_some() && mode != "figure-width" {
            // These two local shaping boundaries must exceed the thirteen
            // source-flow slots now required before shaping can start. Keep
            // the shaping ceiling exact; add glyphs rather than slack to it.
            if mode.starts_with("shape-") { "本文本文" } else { "本文" }
        } else {
            "Result"
        };
        let text = if mode == "missing-glyph" {
            format!("{word}\u{10ffff}")
        } else {
            word.into()
        };
        let per_paragraph = 1 + 2 * word.chars().count() as u64;
        let mut base = limits().base().get().clone();
        if mode.starts_with("shape-") {
            base.max_fragments = 2 * per_paragraph - u64::from(mode == "shape-limit");
        }
        let limits = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let mut data = if mode == "figure-width" {
            super::super::figures::figure_data("png", PNG, None)
        } else {
            let mut data = wide_data(&text);
            let p = data["document"]["blocks"][0]["blocks"][0].clone();
            data["document"]["blocks"][0]["blocks"] = json!([p.clone(), p]);
            if mode == "missing-glyph" {
                data["document"]["blocks"][0]["blocks"][0]["children"][0]["text_span"]
                    ["end_byte"] = word.len().into();
                data["document"]["blocks"][0]["blocks"][1]["children"][0]["text_span"]
                    ["start_byte"] = word.len().into();
            }
            data
        };
        if mode == "figure-width" {
            let span = data["document"]["blocks"][0]["span"].clone();
            for _ in 0..3 {
                data["document"]["blocks"][0]["blocks"][0]["children"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"kind":"hard_break","node_id":0,"span":span}));
            }
        }
        super::super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
        if let Some(font) = font {
            data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
        }
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        if let Some(font) = font {
            fs::write(root.0.join("body.bin"), font).unwrap();
        }
        if mode == "figure-width" {
            fs::write(root.0.join("figure.bin"), PNG).unwrap();
        }
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap();
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let mut records = u64::MAX;
        let shape = shape_book_v2_authored_text_counted(
            &policy,
            &flow,
            input.resources(),
            &limits,
            bindings.epoch(),
            None,
            &mut records,
        );
        let shape_records = records;
        let legacy = shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            bindings.epoch(),
            None,
        );
        let preparation_records = if mode == "missing-glyph" || mode == "shape-limit" {
            assert_eq!(shape.err().unwrap(), legacy.err().unwrap());
            assert_eq!(records, per_paragraph, "{mode}");
            records
        } else {
            let shape = shape.unwrap();
            assert_eq!(shape.fingerprint(), legacy.unwrap().fingerprint());
            assert_eq!(records, shape.output_records());
            assert_eq!(records, 2 * per_paragraph);
            if mode == "shape-exact" {
                assert_eq!(records, limits.base().get().max_fragments);
            }
            let prepared = prepare_book_v2_inline_items_with_native_context_counted(
                &flow,
                &shape,
                input.resources(),
                &bindings,
                &limits,
                MODE,
                None,
                &mut records,
            );
            let legacy = prepare_book_v2_inline_items_with_native_context(
                &flow,
                &shape,
                input.resources(),
                &bindings,
                &limits,
                MODE,
                None,
            );
            if mode == "figure-width" {
                assert_eq!(prepared.err().unwrap(), legacy.err().unwrap());
                assert_eq!(records, 4 * word.chars().count() as u64 + 4);
                assert!(records > shape_records);
            } else {
                assert_eq!(
                    prepared.unwrap().fingerprint(),
                    legacy.unwrap().fingerprint()
                );
                assert_eq!(records, 4 * word.chars().count() as u64);
            }
            let inline_records = records;
            if mode == "success" {
                // Context history is accepted before paragraph endpoint checks.
                let bad = [
                    ProductionParagraphLineContext {
                        owner: flow.paragraphs()[0].owner(),
                        ends: &[1],
                    },
                    ProductionParagraphLineContext {
                        owner: flow.paragraphs()[1].owner(),
                        ends: &[0],
                    },
                ];
                let cause = shape_book_v2_authored_text_counted(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    bindings.epoch(),
                    Some(&bad),
                    &mut records,
                )
                .unwrap_err();
                assert_eq!(records, 4);
                assert_eq!(cause.kind, ProductionTextShapeErrorKind::InvalidLineContext);
                let foreign = [
                    ProductionParagraphLineContext {
                        owner: flow.paragraphs()[0].owner(),
                        ends: &[],
                    },
                    ProductionParagraphLineContext {
                        owner: typaxis_core::NodeId::new(999),
                        ends: &[],
                    },
                ];
                assert!(shape_book_v2_authored_text_counted(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    bindings.epoch(),
                    Some(&foreign),
                    &mut records
                )
                .is_err());
                assert_eq!(records, 1);
                assert!(shape_book_v2_authored_text_counted(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    [0; 32],
                    None,
                    &mut records
                )
                .is_err());
                assert_eq!(records, 0);
            }
            inline_records
        };
        if ["missing-glyph", "shape-limit", "figure-width"].contains(&mode) {
            let mut plan_work = 0;
            let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
                &flow,
                &mut plan_work,
                1_000_000,
                0,
                0,
            )
            .unwrap();
            let mut budget =
                BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
            for _ in 0..2 {
                assert!(budgeted(
                    &policy,
                    &flow,
                    input.resources(),
                    &bindings,
                    &limits,
                    MODE,
                    plan.measurement_body(),
                    None,
                    &mut budget,
                    Some(&plan),
                    None,
                    |_| panic!("failed source preparation must not enter the callback")
                )
                .is_err());
                assert_eq!(
                    budget.record_charge(),
                    shape_records.max(preparation_records)
                );
                assert_eq!((budget.candidate_steps(), budget.reshape_passes()), (0, 0));
            }
            let mut seed_budget = typaxis_layout::book_v2::BookV2LineVariantBudget::new(
                1_000_000,
                limits.base().get().max_line_reshape_passes,
            );
            for prior in [0, 3] {
                let history = seed_budget.record_charge().max(prior);
                let mut independent = BookV2BodyLineBudget::new_with_source_records(
                    1_000_000, limits.base().get().max_line_reshape_passes,
                    history, limits.base().get().max_fragments,
                );
                assert!(budgeted(
                    &policy, &flow, input.resources(), &bindings, &limits, MODE,
                    plan.measurement_body(), None, &mut independent, Some(&plan), None, |_| (),
                ).is_err());
                assert!(
                    typaxis_layout::book_v2::prepare_budgeted_book_v2_body_line_variant_seed(
                        &policy,
                        &flow,
                        input.resources(),
                        &bindings,
                        &limits,
                        MODE,
                        plan.measurement_body(),
                        &mut seed_budget,
                        prior,
                        None,
                        Some(&plan),
                        None
                    )
                    .is_err()
                );
                assert_eq!(
                    seed_budget.record_charge(),
                    independent.source_record_charge() + independent.record_charge()
                );
                assert_eq!(
                    (seed_budget.work_steps(), seed_budget.reshape_passes()),
                    (0, 0)
                );
            }
            if mode != "shape-limit" {
                let prefix_work = plan_work
                    + flow
                        .paragraphs()
                        .iter()
                        .map(|p| 1 + p.items().len() as u64)
                        .sum::<u64>();
                let mut driver =
                    crate::book_v2_resources::BookV2PdfConvergenceBudget::new(&limits, 1_000_000);
                for attempt in 1..=2 {
                    let history = driver.observation().record_charge()
                        + command_source_record_charge(&flow) + plan.record_charge();
                    let mut independent = BookV2BodyLineBudget::new_with_source_records(
                        1_000_000, limits.base().get().max_line_reshape_passes,
                        history, limits.base().get().max_fragments,
                    );
                    assert!(budgeted(
                        &policy, &flow, input.resources(), &bindings, &limits, MODE,
                        plan.measurement_body(), None, &mut independent, Some(&plan), None, |_| (),
                    ).is_err());
                    let cause = crate::book_v2_resources::with_budgeted_book_v2_pdf(
                        &input,
                        &limits,
                        MODE,
                        &mut driver,
                        |_, _| panic!("failed source preparation must not paint"),
                    )
                    .unwrap_err();
                    assert!(
                        matches!(
                            cause,
                            crate::book_v2_resources::BookV2ConvergenceError::Stage {
                                stage: "line feedback",
                                ..
                            }
                        ),
                        "{cause:?}"
                    );
                    assert_eq!(
                        driver.observation().record_charge(),
                        independent.source_record_charge() + independent.record_charge()
                    );
                    assert_eq!(driver.observation().work_steps(), attempt * prefix_work);
                    assert_eq!(driver.observation().line_reshape_passes(), 0);
                }
            }
        }
    }
}
