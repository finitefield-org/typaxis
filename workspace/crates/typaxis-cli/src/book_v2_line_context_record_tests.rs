use super::*;
use typaxis_layout::book_v2::{
    prepare_book_v2_footnote_lines, prepare_book_v2_footnote_lines_counted,
    prepare_budgeted_book_v2_body_line_variant_seed, BookV2LineVariantBudget,
};

#[test]
fn book_v2_line_context_records_preserve_partial_capture_and_projection() {
    check(None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_line_context_records_preserve_original_harano_capture() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&bytes));
}
fn check(font: Option<&[u8]>) {
    for notes in [false, true] {
        let root = Root::new();
        let limits = limits();
        let text = if font.is_some() { "本文" } else { "Result" };
        let mut data = wide_data(text);
        let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
        data["document"]["blocks"][0]["blocks"] =
            json!([paragraph.clone(), paragraph.clone(), paragraph.clone()]);
        if notes {
            let span = data["document"]["blocks"][0]["span"].clone();
            for (index, id) in ["z-note", "a-note", "z-note"].into_iter().enumerate() {
                data["document"]["blocks"][0]["blocks"][index]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id}));
            }
            data["document"]["footnotes"] = json!([
                {"node_id":0,"span":span,"footnote_id":"z-note","blocks":[paragraph.clone()]},
                {"node_id":0,"span":span,"footnote_id":"a-note","blocks":[paragraph.clone()]}
            ]);
            data["page_masters"]["masters"][0]["body"]["height"] = 8_000_000.into();
            data["page_masters"]["masters"][0]["footnote"] =
                json!({"x":500000,"y":9000000,"width":10000000,"height":2000000});
        }
        super::super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
        let input = if let Some(font) = font {
            vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let maximum = 100_000_000;
        let mut plan_work = 0;
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
            &flow,
            &mut plan_work,
            maximum,
            0,
            0,
        )
        .unwrap();
        assert!(!plan.requires_width_reflow());
        let passes = limits.base().get().max_line_reshape_passes;
        let run = |allowance: &mut BookV2BodyLineBudget| {
            budgeted(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                MODE,
                plan.measurement_body(),
                None,
                allowance,
                Some(&plan),
                None,
                |stable| {
                    let lines = stable.lines();
                    let mut records = u64::MAX;
                    let contexts = lines.selected_line_contexts_counted(&mut records).unwrap();
                    let context_records = lines.output_records()
                        + lines
                            .paragraphs()
                            .iter()
                            .map(|p| p.lines().len() as u64 + 2)
                            .sum::<u64>();
                    assert_eq!(records, context_records);
                    assert_eq!(
                        contexts.source_fingerprint(),
                        lines.selected_line_contexts().unwrap().source_fingerprint()
                    );
                    let footnotes =
                        prepare_book_v2_footnote_lines_counted(lines, &limits, &mut records)
                            .unwrap();
                    assert_eq!(records, lines.output_records() + if notes { 10 } else { 0 });
                    assert_eq!(
                        records,
                        prepare_book_v2_footnote_lines(lines, &limits)
                            .unwrap()
                            .record_charge()
                    );
                    assert_eq!(footnotes.definitions().len(), if notes { 2 } else { 0 });
                    assert_eq!(footnotes.references().len(), if notes { 3 } else { 0 });
                    let mut changed = limits.base().get().clone();
                    changed.max_pages -= 1;
                    let changed = M4EffectiveResourceLimits::new(
                        ValidatedResourceLimits::new(changed).unwrap(),
                        M4ResourceLimits::default(),
                    )
                    .unwrap();
                    let error =
                        prepare_book_v2_footnote_lines_counted(lines, &changed, &mut records)
                            .err()
                            .unwrap();
                    assert_eq!(records, lines.output_records());
                    assert_eq!(
                        error,
                        prepare_book_v2_footnote_lines(lines, &changed)
                            .err()
                            .unwrap()
                    );
                    (
                        context_records.max(footnotes.record_charge()),
                        footnotes.record_charge(),
                    )
                },
            )
        };
        let mut full = BookV2BodyLineBudget::new(maximum, passes);
        let (retained, rebuilt) = run(&mut full).unwrap();
        assert!(full.record_charge() >= retained);
        let mut short = BookV2BodyLineBudget::new(full.candidate_steps() - 1, passes);
        assert!(run(&mut short).is_err());
        assert!(short.record_charge() > 0);
        assert!(short.record_charge() <= full.record_charge());
        let before = short.record_charge();
        assert!(run(&mut short).is_err());
        assert!(short.record_charge() >= before);
        let mut seed_budget = BookV2LineVariantBudget::new(maximum, passes);
        let seed = |budget: &mut BookV2LineVariantBudget, prior| {
            prepare_budgeted_book_v2_body_line_variant_seed(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                MODE,
                plan.measurement_body(),
                budget,
                prior,
                None,
                Some(&plan),
                None,
            )
        };
        let complete = seed(&mut seed_budget, 0).unwrap();
        assert_eq!(seed_budget.record_charge(), complete.record_charge());
        let extra = complete.record_charge() - rebuilt;
        let cap = limits.base().get().max_fragments;
        let mut partial = false;
        for remaining in 0..extra {
            let prior = cap - remaining;
            let mut budget = BookV2LineVariantBudget::new(maximum, passes);
            assert!(seed(&mut budget, prior).is_err());
            assert!(budget.record_charge() >= prior && budget.record_charge() <= cap);
            partial |= budget.record_charge() > prior;
            if remaining > 0 {
                assert_eq!(budget.work_steps(), complete.work_steps());
            }
        }
        assert!(
            partial,
            "later paragraphs must retain earlier accepted contexts"
        );
        let mut exact = BookV2LineVariantBudget::new(maximum, passes);
        assert_eq!(seed(&mut exact, cap - extra).unwrap().record_charge(), cap);
        assert_eq!(exact.record_charge(), cap);
        // Independent line attempt versus the driver before its callback.
        let mut local = BookV2BodyLineBudget::new(full.candidate_steps() - 1, passes);
        let cause = run(&mut local).unwrap_err();
        let prefix_work = plan_work
            + flow
                .paragraphs()
                .iter()
                .map(|p| 1 + p.items().len() as u64)
                .sum::<u64>();
        let mut driver = crate::book_v2_resources::BookV2PdfConvergenceBudget::new(
            &limits,
            prefix_work + full.candidate_steps() - 1,
        );
        let result = crate::book_v2_resources::with_budgeted_book_v2_pdf(
            &input,
            &limits,
            MODE,
            &mut driver,
            |_, _| panic!("failed line capture reached PDF"),
        );
        let crate::book_v2_resources::BookV2ConvergenceError::Stage { stage, source } =
            result.unwrap_err()
        else {
            panic!("expected line failure")
        };
        assert_eq!(stage, "line feedback");
        assert_eq!(
            format!(
                "{:?}",
                source
                    .downcast_ref::<typaxis_layout::ProductionBodyReshapeError>()
                    .unwrap()
            ),
            format!("{cause:?}")
        );
        assert_eq!(
            driver.observation().record_charge(),
            command_source_record_charge(&flow) + plan.record_charge() + local.record_charge()
        );
        assert_eq!(
            driver.observation().work_steps(),
            prefix_work + local.candidate_steps()
        );
        assert_eq!(
            driver.observation().line_reshape_passes(),
            local.reshape_passes()
        );
        assert_eq!(driver.observation().page_passes(), 0);
    }
}
