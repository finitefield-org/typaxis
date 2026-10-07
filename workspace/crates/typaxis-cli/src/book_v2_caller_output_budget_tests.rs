use super::*;
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, layout_book_v2_body_inline_lines,
    prepare_book_v2_inline_items_with_native_context_counted,
    prepare_book_v2_inline_items_with_source_budget_counted,
};
use typaxis_layout::{ProductionBodyReshapeError, ProductionInlinePreparationErrorKind};
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_shaping::book_v2::shape_book_v2_authored_text_with_source_budget_counted;
use typaxis_syntax::book_v2::BookV2SourceVerificationBudget;

#[test]
fn book_v2_caller_output_budget_bounds_actual_shapes_inlines_and_driver() {
    check(None, "Result");
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_caller_output_budget_bounds_original_harano_output() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font), "本文");
}

fn fixture(text: &str) -> Value {
    let mut data = super::body_line_budget::wide_data(text);
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    data["document"]["blocks"][0]["blocks"] = json!([paragraph.clone(), paragraph]);
    let span = data["document"]["blocks"][0]["span"].clone();
    for _ in 0..8 {
        data["document"]["blocks"][0]["blocks"][1]["children"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"soft_break","node_id":0,"span":span}));
    }
    super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    data
}

fn check(font: Option<&[u8]>, text: &str) {
    let limits = limits();
    let root = Root::new();
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, fixture(text), &limits, font, text.as_bytes())
    } else {
        prepared(&root, fixture(text), text.as_bytes(), &limits)
    };
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let source_charge = 1
        + flow.events().len() as u64
        + flow.paragraphs().len() as u64
        + flow
            .paragraphs()
            .iter()
            .map(|p| p.items().len() as u64)
            .sum::<u64>();
    assert_eq!(source_charge, flow.source_record_charge());
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let epoch = bindings.epoch();
    let mode = JapaneseLineBreakMode::Normal;
    let plain =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, epoch, None)
            .unwrap();
    let mut inline_charge = 0;
    let inline = prepare_book_v2_inline_items_with_native_context_counted(
        &flow,
        &plain,
        input.resources(),
        &bindings,
        &limits,
        mode,
        None,
        &mut inline_charge,
    )
    .unwrap();
    let scalar_records = 2 * text.chars().count() as u64;
    assert_eq!(inline_charge, 2 * scalar_records + 8);
    assert!(inline_charge > plain.output_records());
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut 0,
        100_000_000,
        0,
        0,
    )
    .unwrap();
    let lines =
        layout_book_v2_body_inline_lines(&inline, plan.measurement_body(), 100_000_000).unwrap();
    let contexts = lines.selected_line_contexts().unwrap();
    let inputs: Vec<_> = contexts
        .paragraphs()
        .iter()
        .map(|p| ProductionParagraphLineContext {
            owner: p.owner(),
            ends: p.ends(),
        })
        .collect();
    for contexts in [None, Some(inputs.as_slice())] {
        let reference = shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            epoch,
            contexts,
        )
        .unwrap();
        let actual = contexts.map_or(0, |c| c.iter().map(|p| p.ends.len() as u64 + 1).sum())
            + reference
                .paragraphs()
                .iter()
                .flat_map(|p| p.runs())
                .map(|r| {
                    let run = r.glyph_run();
                    1 + run.glyphs.len() as u64 + run.clusters.len() as u64
                })
                .sum::<u64>();
        assert_eq!(actual, reference.output_records());
        let mut last = 0;
        for available in 0..=actual {
            let maximum = 7 + source_charge + available;
            let mut budget = BookV2SourceVerificationBudget::new(7, maximum);
            let mut observed = u64::MAX;
            let result = shape_book_v2_authored_text_with_source_budget_counted(
                &policy,
                &flow,
                input.resources(),
                &limits,
                epoch,
                contexts,
                &mut budget,
                &mut observed,
            );
            assert_eq!(budget.record_charge(), 7 + source_charge);
            assert!(last <= observed && observed <= available);
            last = observed;
            assert!(budget.record_charge() + observed <= maximum);
            if available == actual {
                assert_eq!(result.unwrap().fingerprint(), reference.fingerprint());
                assert_eq!(observed, actual);
            } else {
                assert_eq!(
                    result.unwrap_err().kind,
                    ProductionTextShapeErrorKind::OutputLimit
                );
                let prior = budget.record_charge();
                assert!(shape_book_v2_authored_text_with_source_budget_counted(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    epoch,
                    contexts,
                    &mut budget,
                    &mut observed,
                )
                .is_err());
                assert!(budget.record_charge() >= prior && budget.record_charge() <= maximum);
            }
        }
        // Exact prepaid output credit is already part of caller history.
        let maximum = 7 + actual + source_charge;
        let mut prepaid =
            BookV2SourceVerificationBudget::new_with_prepaid_records(7 + actual, maximum, actual);
        let mut observed = 0;
        let accepted = shape_book_v2_authored_text_with_source_budget_counted(
            &policy,
            &flow,
            input.resources(),
            &limits,
            epoch,
            contexts,
            &mut prepaid,
            &mut observed,
        )
        .unwrap();
        assert_eq!(accepted.fingerprint(), reference.fingerprint());
        assert_eq!((prepaid.record_charge(), observed), (maximum, actual));
    }
    let mut partial = false;
    for available in plain.output_records()..=inline_charge {
        let maximum = 7 + source_charge + available;
        let mut budget = BookV2SourceVerificationBudget::new(7, maximum);
        let mut observed = u64::MAX;
        let result = prepare_book_v2_inline_items_with_source_budget_counted(
            &flow,
            &plain,
            input.resources(),
            &bindings,
            &limits,
            mode,
            None,
            &mut budget,
            &mut observed,
        );
        assert_eq!(budget.record_charge(), 7 + source_charge);
        assert!(observed <= available && budget.record_charge() + observed <= maximum);
        if available == inline_charge {
            assert_eq!(result.unwrap().fingerprint(), inline.fingerprint());
            assert_eq!(observed, inline_charge);
        } else {
            assert_eq!(
                result.err().unwrap().kind,
                ProductionInlinePreparationErrorKind::UnitLimit
            );
            partial |= observed > 0;
        }
    }
    assert!(partial);
    check_markers(font, text);
    check_driver(font, text);
}

fn check_markers(font: Option<&[u8]>, text: &str) {
    use typaxis_layout::book_v2::prepare_book_v2_footnote_lines_counted;
    let limits = limits();
    let root = Root::new();
    let mut data = super::body_line_budget::wide_data(text);
    let span = data["document"]["blocks"][0]["span"].clone();
    let mut paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let definition = paragraph.clone();
    paragraph["children"].as_array_mut().unwrap().push(json!({
        "kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"
    }));
    data["document"]["blocks"][0]["blocks"] = json!([{
        "kind":"list","node_id":0,"classes":[],"span":span,"ordered":true,"start":1,
        "items":[{"node_id":0,"span":span,"blocks":[paragraph]}]
    }]);
    data["document"]["footnotes"] = json!([{
        "node_id":0,"span":span,"footnote_id":"note","blocks":[definition]
    }]);
    let mut rule = data["style_sheet"]["rules"][2].clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "list-text".into();
    rule["source_order"] = 3.into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    let master = &mut data["page_masters"]["masters"][0];
    master["height"] = 24_000_000.into();
    master["trim"]["height"] = 24_000_000.into();
    master["body"]["height"] = 18_000_000.into();
    master["footnote"] = json!({"x":500_000,"y":20_000_000,"width":10_000_000,"height":2_000_000});
    super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let plain = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let costs: Vec<_> = plain
        .paragraphs()
        .iter()
        .flat_map(|p| p.runs())
        .map(|r| r.glyph_run())
        .chain(plain.list_markers().iter().map(|m| m.glyph_run()))
        .chain(plain.footnote_markers().iter().map(|m| m.glyph_run()))
        .map(|r| 1 + r.glyphs.len() as u64 + r.clusters.len() as u64)
        .collect();
    assert_eq!(
        (plain.list_markers().len(), plain.footnote_markers().len()),
        (1, 1)
    );
    let actual = costs.iter().sum::<u64>();
    assert_eq!(plain.output_records(), actual);
    for available in 0..=actual {
        let mut budget =
            BookV2SourceVerificationBudget::new(7, 7 + flow.source_record_charge() + available);
        let mut observed = 0;
        let result = shape_book_v2_authored_text_with_source_budget_counted(
            &policy,
            &flow,
            input.resources(),
            &limits,
            bindings.epoch(),
            None,
            &mut budget,
            &mut observed,
        );
        let mut prefix = 0;
        for cost in &costs {
            if prefix + cost > available {
                break;
            }
            prefix += cost;
        }
        assert_eq!(observed, prefix);
        assert_eq!(budget.record_charge(), 7 + flow.source_record_charge());
        if available == actual {
            assert_eq!(result.unwrap().fingerprint(), plain.fingerprint());
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ProductionTextShapeErrorKind::OutputLimit
            );
        }
    }
    let mode = JapaneseLineBreakMode::Normal;
    let mut inline_records = 0;
    let inline = prepare_book_v2_inline_items_with_native_context_counted(
        &flow,
        &plain,
        input.resources(),
        &bindings,
        &limits,
        mode,
        None,
        &mut inline_records,
    )
    .unwrap();
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut 0,
        100_000_000,
        0,
        0,
    )
    .unwrap();
    let selected =
        layout_book_v2_body_inline_lines(&inline, plan.measurement_body(), 100_000_000).unwrap();
    let mut footnote_records = 0;
    let notes =
        prepare_book_v2_footnote_lines_counted(&selected, &limits, &mut footnote_records).unwrap();
    assert_eq!(footnote_records, selected.output_records() + 3);
    assert!(footnote_records > plain.output_records().max(inline_records));
    for short in [false, true] {
        let bound = footnote_records - u64::from(short);
        let mut budget =
            BookV2SourceVerificationBudget::new(7, 7 + flow.source_record_charge() + bound);
        let bounded = prepare_book_v2_inline_items_with_source_budget_counted(
            &flow,
            &plain,
            input.resources(),
            &bindings,
            &limits,
            mode,
            None,
            &mut budget,
            &mut inline_records,
        )
        .unwrap();
        let selected =
            layout_book_v2_body_inline_lines(&bounded, plan.measurement_body(), 100_000_000)
                .unwrap();
        let mut observed = 0;
        let result = prepare_book_v2_footnote_lines_counted(&selected, &limits, &mut observed);
        if short {
            assert_eq!(
                result.err().unwrap().kind,
                ProductionInlinePreparationErrorKind::UnitLimit
            );
            assert_eq!(observed, selected.output_records());
        } else {
            let bounded_notes = result.unwrap();
            assert_eq!(observed, footnote_records);
            assert_eq!(bounded_notes.definitions().len(), notes.definitions().len());
            assert_eq!(bounded_notes.references().len(), notes.references().len());
        }
    }
}

fn check_driver(font: Option<&[u8]>, text: &str) {
    use crate::book_v2_resources::{
        with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
    };
    let original = limits();
    let make = |root: &Root, limits: &M4EffectiveResourceLimits| {
        if let Some(font) = font {
            vector_tests::vector_input_with_font(root, fixture(text), limits, font, text.as_bytes())
        } else {
            prepared(root, fixture(text), text.as_bytes(), limits)
        }
    };
    let root = Root::new();
    let input = make(&root, &original);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let source = flow.source_record_charge();
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
        &flow,
        &mut 0,
        100_000_000,
        0,
        0,
    )
    .unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &original).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &original).unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &original,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let first = shape.paragraphs()[0]
        .runs()
        .iter()
        .map(|r| {
            let run = r.glyph_run();
            1 + run.glyphs.len() as u64 + run.clusters.len() as u64
        })
        .sum::<u64>();
    let initial = command_source_record_charge(&flow) + plan.record_charge();
    for available in [0, first, shape.output_records() - 1] {
        let cap = initial + source + available;
        let mut base = original.base().get().clone();
        base.max_fragments = cap;
        let limits = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        let root = Root::new();
        let input = make(&root, &limits);
        let mut budget = BookV2PdfConvergenceBudget::new(&limits, 100_000_000);
        let error = with_budgeted_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| panic!("retained shape must be bounded before PDF construction"),
        )
        .err()
        .unwrap();
        let BookV2ConvergenceError::Stage { stage, source } = error else {
            panic!("{error:?}")
        };
        assert_eq!(stage, "line feedback");
        assert!(
            matches!(source.downcast_ref::<ProductionBodyReshapeError>().unwrap(),
            ProductionBodyReshapeError::Shape(e) if e.kind == ProductionTextShapeErrorKind::OutputLimit)
        );
        let accepted = if available >= first { first } else { 0 };
        let observed = budget.observation();
        assert_eq!(
            observed.record_charge(),
            initial + flow.source_record_charge() + accepted
        );
        assert_eq!(
            (
                observed.line_reshape_passes(),
                observed.page_passes(),
                observed.output_charge()
            ),
            (0, 0, 0)
        );
        assert!(with_budgeted_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut budget,
            |_, _| ()
        )
        .is_err());
        assert!(budget.observation().record_charge() >= observed.record_charge());
        assert!(budget.observation().record_charge() <= cap);
    }
}
