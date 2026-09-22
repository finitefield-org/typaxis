use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    prepare_book_v2_table_search_counted,
};

#[test]
fn book_v2_table_constructor_budget_retains_nested_failures() {
    check(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_table_constructor_budget_retains_original_harano_failures() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font));
}

fn check(font: Option<&[u8]>) {
    for mode in ["forced", "two-children", "deep", "definition-query"] {
        let root = Root::new();
        let limits = limits();
        let (text, split) = if font.is_some() {
            ("本文の柱", 6)
        } else {
            ("LeftRight", 4)
        };
        let data = nested_tables::nested(text, split, mode);
        let input = if let Some(font) = font {
            vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        let raw = |v| Length::from_raw(v).unwrap();
        let rect = Rect::new(
            raw(10 * 65536),
            raw(10 * 65536),
            PositiveLength::new(raw(if mode == "deep" { 780 } else { 180 } * 65536)).unwrap(),
            PositiveLength::new(raw(48 * 65536)).unwrap(),
        );
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            rect,
            1_000_000,
            |stable| {
                let measured = prepare_book_v2_table_measurements(
                    prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                        .unwrap(),
                    &limits,
                )
                .unwrap();
                let initial = measured.record_charge();
                let original =
                    prepare_book_v2_table_search(&measured, 0, &limits, 1_000_000, 0).unwrap();
                let full_work = original.work_charge();
                let full_records = original.record_charge();
                assert!(full_work > 10 && full_work < 10_000);
                let mut records = u64::MAX;
                let mut work = u64::MAX;
                let exact = prepare_book_v2_table_search_counted(
                    &measured,
                    0,
                    &limits,
                    full_work,
                    0,
                    &mut records,
                    &mut work,
                )
                .unwrap();
                assert_eq!((records, work), (full_records, full_work));
                assert_eq!(exact.header_height(), original.header_height());
                assert_eq!(exact.maximum_height(), original.maximum_height());
                // The exact constructor allowance is exhausted. Give fragment
                // selection its own allowance when comparing successful output.
                let mut exact = prepare_book_v2_table_search_counted(
                    &measured,
                    0,
                    &limits,
                    1_000_000,
                    0,
                    &mut records,
                    &mut work,
                )
                .unwrap();
                let mut legacy = original;
                let a = exact.begin().unwrap();
                let b = legacy.begin().unwrap();
                let capacity = exact.maximum_height();
                assert_eq!(
                    exact
                        .evaluate(&a, capacity)
                        .unwrap()
                        .map(|s| s.fingerprint()),
                    legacy
                        .evaluate(&b, capacity)
                        .unwrap()
                        .map(|s| s.fingerprint())
                );
                // Walk every work boundary, including inside recursively prepared
                // children. No failure may replace a real partial ledger with
                // zero work or a fictitious exhausted record allowance.
                let mut previous_work = 0;
                let mut previous_records = initial;
                for cap in 0..full_work {
                    let result = prepare_book_v2_table_search_counted(
                        &measured,
                        0,
                        &limits,
                        cap,
                        0,
                        &mut records,
                        &mut work,
                    );
                    assert_eq!(
                        result.err().unwrap().kind,
                        typaxis_pagination::ProductionBodyPaginationErrorKind::TableSearchLimit,
                        "{mode}: {cap}"
                    );
                    assert!(work >= previous_work && work <= cap, "{mode}: {cap}/{work}");
                    assert!(
                        records >= previous_records && records <= full_records,
                        "{mode}: {cap}/{records}"
                    );
                    previous_work = work;
                    previous_records = records;
                }
                assert!(previous_work > full_work / 2);
                let retained = records;
                let cap = full_work / 2;
                assert!(prepare_book_v2_table_search_counted(
                    &measured,
                    0,
                    &limits,
                    cap,
                    retained,
                    &mut records,
                    &mut work,
                )
                .is_err());
                assert!(records > retained && work > 0 && work <= cap);
                let maximum = limits.base().get().max_fragments;
                let exact_prior = maximum - (full_records - initial);
                let exact_records = prepare_book_v2_table_search_counted(
                    &measured,
                    0,
                    &limits,
                    full_work,
                    exact_prior,
                    &mut records,
                    &mut work,
                )
                .unwrap();
                assert_eq!(records, maximum);
                assert_eq!(work, full_work);
                assert_eq!(exact_records.record_charge(), records);
                assert!(prepare_book_v2_table_search_counted(
                    &measured,
                    0,
                    &limits,
                    full_work,
                    exact_prior + 1,
                    &mut records,
                    &mut work,
                )
                .is_err());
                assert!(records <= maximum && records >= exact_prior + 1);
                assert!(work <= full_work);
                assert!(prepare_book_v2_table_search_counted(
                    &measured,
                    usize::MAX,
                    &limits,
                    full_work,
                    0,
                    &mut records,
                    &mut work,
                )
                .is_err());
                assert_eq!((records, work), (initial, 0));
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}
