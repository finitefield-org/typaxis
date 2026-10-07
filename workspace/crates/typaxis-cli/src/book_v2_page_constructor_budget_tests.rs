use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_body_search_counted, prepare_book_v2_table_measurements,
};
use typaxis_pagination::ProductionBodyPaginationErrorKind as E;

#[test]
fn book_v2_page_constructor_budget_retains_body_and_definition_failures() {
    check(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_page_constructor_budget_retains_original_harano_failures() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&bytes));
}

fn check(font: Option<&[u8]>) {
    let maximum = 100_000_000;
    for mode in ["empty", "two-children", "definition-query", "joint"] {
        let root = Root::new();
        let limits = limits();
        let (text, split) = if font.is_some() {
            ("本文の柱", 6)
        } else {
            ("LeftRight", 4)
        };
        let input = if mode == "empty" {
            input(&root, text, font, &limits)
        } else {
            let mut data = super::super::nested_tables::nested(
                text,
                split,
                if mode == "joint" {
                    "definition-query"
                } else {
                    mode
                },
            );
            if mode == "joint" {
                let table = data["document"]["footnotes"][0]["blocks"][0].clone();
                data["document"]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .push(table);
                let mut note = data["document"]["footnotes"][0].clone();
                note["footnote_id"] = "second".into();
                data["document"]["footnotes"]
                    .as_array_mut()
                    .unwrap()
                    .push(note);
                super::super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
            }
            if let Some(font) = font {
                vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
            } else {
                prepared(&root, data, text.as_bytes(), &limits)
            }
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let mut pre_work = 0;
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
            &flow,
            &mut pre_work,
            maximum,
            0,
            0,
        )
        .unwrap();
        assert!(!plan.requires_width_reflow());
        pre_work += flow
            .paragraphs()
            .iter()
            .map(|p| 1 + p.items().len() as u64)
            .sum::<u64>();
        let mut allowance =
            BookV2BodyLineBudget::new(maximum, limits.base().get().max_line_reshape_passes);
        budgeted(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            MODE,
            plan.measurement_body(),
            None,
            &mut allowance,
            Some(&plan),
            None,
            |lines| {
                let prefix = pre_work + lines.candidate_steps();
                let prior = command_source_record_charge(&flow)
                    + plan.record_charge() + lines.source_record_charge() + lines.retained_record_charge();
                let measured = prepare_book_v2_table_measurements(
                    prepare_book_v2_body_flow(
                        lines.lines(),
                        None,
                        lines.footnotes(),
                        &limits,
                        prior,
                    )
                    .unwrap(),
                    &limits,
                )
                .unwrap();
                let initial = measured.record_charge();
                let original =
                    prepare_book_v2_table_body_search(&measured, &limits, maximum, initial)
                        .unwrap();
                let full_work = original.work_steps();
                let full_records = original.record_charge();
                assert!(full_work < 10_000, "{mode}: {full_work}");
                assert_eq!(full_work == 0, mode == "empty");
                let mut records = 0;
                let mut work = 0;
                let exact = prepare_book_v2_table_body_search_counted(
                    &measured,
                    &limits,
                    full_work,
                    initial,
                    &mut records,
                    &mut work,
                )
                .unwrap();
                assert_eq!((records, work), (full_records, full_work));
                assert_eq!(exact.body_table_count(), original.body_table_count());
                let mut caps: Vec<_> = if full_work == 0 {
                    Vec::new()
                } else if font.is_none() {
                    (0..full_work).collect()
                } else {
                    (0..32).map(|i| (full_work - 1) * i / 31).collect()
                };
                caps.sort_unstable();
                caps.dedup();
                let mut last = (initial, 0);
                let mut kinds = std::collections::BTreeSet::new();
                for cap in caps {
                    let error = prepare_book_v2_table_body_search_counted(
                        &measured,
                        &limits,
                        cap,
                        initial,
                        &mut records,
                        &mut work,
                    )
                    .err()
                    .unwrap();
                    assert!(
                        matches!(error.kind, E::TableSearchLimit | E::FootnoteSearchLimit),
                        "{mode}: {cap}: {error:?}"
                    );
                    kinds.insert(format!("{:?}", error.kind));
                    assert!(
                        records >= last.0 && records <= full_records,
                        "{mode}: {cap}/{records}"
                    );
                    assert!(work >= last.1 && work <= cap, "{mode}: {cap}/{work}");
                    last = (records, work);
                }
                if mode != "empty" {
                    assert!(kinds.contains("TableSearchLimit"));
                }
                if full_work > 0 {
                    assert!(kinds.contains("FootnoteSearchLimit"));
                }
                let record_cap = limits.base().get().max_fragments;
                let exact_prior = record_cap - (full_records - initial);
                assert!(prepare_book_v2_table_body_search_counted(
                    &measured,
                    &limits,
                    full_work,
                    exact_prior,
                    &mut records,
                    &mut work,
                )
                .is_ok());
                assert_eq!((records, work), (record_cap, full_work));
                assert_eq!(
                    prepare_book_v2_table_body_search_counted(
                        &measured,
                        &limits,
                        full_work,
                        exact_prior + 1,
                        &mut records,
                        &mut work,
                    )
                    .err()
                    .unwrap()
                    .kind,
                    E::FragmentLimit
                );
                assert!(records >= exact_prior + 1 && records <= record_cap);

                let driver_caps = if full_work == 0 {
                    Vec::new()
                } else {
                    vec![0, full_work / 2, full_work - 1]
                };
                for cap in driver_caps {
                    let cause = prepare_book_v2_table_body_search_counted(
                        &measured,
                        &limits,
                        cap,
                        initial,
                        &mut records,
                        &mut work,
                    )
                    .err()
                    .unwrap();
                    let mut budget = BookV2PdfConvergenceBudget::new(&limits, prefix + cap);
                    let error =
                        with_budgeted_book_v2_pdf(&input, &limits, MODE, &mut budget, |_, _| {
                            panic!("failed constructor reached PDF consumer")
                        })
                        .unwrap_err();
                    let BookV2ConvergenceError::Stage { stage, source } = error else {
                        panic!("{error:?}")
                    };
                    assert_eq!(stage, "page search", "{mode}");
                    assert_eq!(
                        *source
                            .downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>()
                            .unwrap(),
                        cause
                    );
                    let observed = budget.observation();
                    assert_eq!(observed.work_steps(), prefix + work, "{mode}: {cap}");
                    assert_eq!(observed.record_charge(), records, "{mode}: {cap}");
                    assert_eq!(observed.spool_charge(), plan.spool_charge());
                    assert_eq!(observed.output_charge(), 0);
                    assert_eq!(observed.page_passes(), 0);
                    assert_eq!(
                        observed.line_reshape_passes() as usize,
                        lines.passes().len()
                    );
                    assert_eq!(observed.candidate_passes(), 0);
                    assert!(with_budgeted_book_v2_pdf(
                        &input,
                        &limits,
                        MODE,
                        &mut budget,
                        |_, _| ()
                    )
                    .is_err());
                    assert!(budget.observation().work_steps() >= observed.work_steps());
                    assert!(budget.observation().work_steps() <= prefix + cap);
                    assert!(budget.observation().record_charge() >= observed.record_charge());
                }
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}
