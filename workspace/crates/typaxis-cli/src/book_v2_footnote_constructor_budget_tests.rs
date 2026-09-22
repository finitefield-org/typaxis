use super::*;
use typaxis_pagination::book_v2::*;
use typaxis_pagination::{ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as E};

#[test]
fn book_v2_footnote_constructor_budget_retains_standalone_failures() {
    check(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_footnote_constructor_budget_retains_original_harano_failures() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&bytes));
}

type Observation = (u64, u64);
type Outcome = Result<Observation, ProductionBodyPaginationError>;

fn boundaries(
    name: &str,
    initial: u64,
    maximum_records: u64,
    sampled: bool,
    legacy: impl Fn(u64, u64) -> Outcome,
    counted: impl Fn(u64, u64, &mut u64, &mut u64) -> Outcome,
) {
    let (full_records, full_work) = legacy(100_000_000, initial).unwrap();
    assert!(full_work < 10_000, "{name}: {full_work}");
    assert!(full_records > initial);
    let mut records = u64::MAX;
    let mut work = u64::MAX;
    assert_eq!(
        counted(full_work, initial, &mut records, &mut work).unwrap(),
        (full_records, full_work),
        "{name}"
    );
    assert_eq!((records, work), (full_records, full_work));
    let mut caps: Vec<_> = if full_work == 0 {
        vec![]
    } else if sampled {
        (0..32).map(|i| (full_work - 1) * i / 31).collect()
    } else {
        (0..full_work).collect()
    };
    caps.sort_unstable();
    caps.dedup();
    let mut previous = (initial, 0);
    for cap in caps {
        let error = counted(cap, initial, &mut records, &mut work).unwrap_err();
        assert!(
            matches!(error.kind, E::TableSearchLimit | E::FootnoteSearchLimit),
            "{name}: {cap}: {error:?}"
        );
        assert_eq!(error, legacy(cap, initial).unwrap_err());
        assert!(
            records >= previous.0 && records <= full_records,
            "{name}: {cap}: {records}"
        );
        assert!(work >= previous.1 && work <= cap, "{name}: {cap}: {work}");
        previous = (records, work);
    }
    if full_work > 1 {
        assert!(
            previous.0 > initial && previous.1 > 0,
            "{name}: lost failure prefix"
        );
    }
    let additional = full_records - initial;
    let exact_prior = maximum_records - additional;
    assert_eq!(
        counted(full_work, exact_prior, &mut records, &mut work).unwrap(),
        (maximum_records, full_work)
    );
    assert_eq!((records, work), (maximum_records, full_work));
    // Check every reservation boundary, including the final demand owner and
    // the extra owner between a single table and its note search.
    for remaining in 0..additional {
        let prior = maximum_records - remaining;
        let error = counted(full_work, prior, &mut records, &mut work).unwrap_err();
        assert_eq!(error.kind, E::FragmentLimit, "{name}: {remaining}");
        assert_eq!(error, legacy(full_work, prior).unwrap_err());
        assert!(records >= prior && records <= maximum_records);
        assert!(work <= full_work);
    }
    if full_work > 0 {
        let cap = full_work - 1;
        counted(cap, initial, &mut records, &mut work).unwrap_err();
        let first = (records, work);
        let _ = counted(cap - first.1, first.0, &mut records, &mut work);
        assert!(records >= first.0 && records <= maximum_records);
        assert!(first.1 + work <= cap);
    }
    // Even an already exhausted incoming prefix is reported unchanged.
    assert_eq!(
        counted(full_work, maximum_records + 1, &mut records, &mut work)
            .unwrap_err()
            .kind,
        E::FragmentLimit
    );
    assert_eq!((records, work), (maximum_records + 1, 0));
}

fn check(font: Option<&[u8]>) {
    let maximum = 100_000_000;
    for mode in ["empty", "notes", "definition-query", "multiple"] {
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
                if mode == "multiple" {
                    "definition-query"
                } else {
                    mode
                },
            );
            if mode == "multiple" {
                let mut second = data["document"]["footnotes"][0].clone();
                second["footnote_id"] = "second".into();
                data["document"]["footnotes"]
                    .as_array_mut()
                    .unwrap()
                    .push(second);
                let table = data["document"]["footnotes"][0]["blocks"][0].clone();
                data["document"]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .push(table);
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
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
            &flow, &mut 0, maximum, 0, 0,
        )
        .unwrap();
        let mut allowance =
            BookV2BodyLineBudget::new(maximum, limits.base().get().max_line_reshape_passes);
        budgeted(&policy, &flow, input.resources(), &bindings, &limits, MODE, plan.measurement_body(), None, &mut allowance, Some(&plan), None, |lines| {
            let measured = prepare_book_v2_table_measurements(prepare_book_v2_body_flow(lines.lines(), None, lines.footnotes(), &limits, plan.record_charge() + lines.footnotes().record_charge()).unwrap(), &limits).unwrap();
            let initial = measured.record_charge();
            let max_records = limits.base().get().max_fragments;
            let sampled = font.is_some();
            macro_rules! check_api {
                ($name:literal, $old:ident, $new:ident, $prior:expr, $work:ident, $($arg:expr),+) => {
                    boundaries(concat!($name, " constructor"), $prior, max_records, sampled,
                        |cap, prior| $old($($arg,)+ &limits, cap, prior).map(|s| (s.record_charge(), s.$work())),
                        |cap, prior, records, work| $new($($arg,)+ &limits, cap, prior, records, work).map(|s| (s.record_charge(), s.$work())));
                }
            }
            if mode == "empty" || mode == "notes" {
                check_api!("flat", prepare_book_v2_footnote_search, prepare_book_v2_footnote_search_counted, measured.flow().record_charge(), visited_items, measured.flow());
                check_api!("flat demand", prepare_book_v2_footnote_demand_search, prepare_book_v2_footnote_demand_search_counted, measured.flow().record_charge(), work_steps, measured.flow());
                if mode == "notes" {
                    check_api!("table notes", prepare_book_v2_table_footnote_search, prepare_book_v2_table_footnote_search_counted, initial, work_charge, &measured, 0);
                }
            } else {
                // Flat searches reject definition tables without losing visits
                // already accepted before that source-structure rejection.
                let mut records = 0;
                let mut work = 0;
                let error = prepare_book_v2_footnote_search_counted(measured.flow(), &limits, maximum, initial, &mut records, &mut work).err().unwrap();
                assert_eq!(error.kind, E::PendingRegion("table_footnote_definition"));
                assert!(work > 0);
                let before = (records, work);
                let demand = prepare_book_v2_footnote_demand_search_counted(measured.flow(), &limits, maximum, initial, &mut records, &mut work).err().unwrap();
                assert_eq!(demand, error);
                assert_eq!((records, work), before);
                // The first root definition table follows the optional body tree.
                let index = if mode == "multiple" { 2 } else { 0 };
                check_api!("definition table", prepare_book_v2_definition_table_demand_search, prepare_book_v2_definition_table_demand_search_counted, initial, work_charge, &measured, index);
            }
            check_api!("mixed queue", prepare_book_v2_mixed_footnote_demand_search, prepare_book_v2_mixed_footnote_demand_search_counted, initial, work_steps, &measured);
            if mode != "empty" {
                let index = if mode == "multiple" { 2 } else { 0 };
                let table = prepare_book_v2_table_search(&measured, index, &limits, maximum, initial).unwrap();
                let table_records = table.record_charge();
                let table_work = table.work_charge();
                let mut records = 0;
                let mut work = 0;
                let result = if mode == "notes" {
                    prepare_book_v2_table_footnote_search_counted(&measured, index, &limits, table_work, initial, &mut records, &mut work).map(|_| ())
                } else {
                    prepare_book_v2_definition_table_demand_search_counted(&measured, index, &limits, table_work, initial, &mut records, &mut work).map(|_| ())
                };
                assert_eq!(result.unwrap_err().kind, E::FootnoteSearchLimit);
                assert_eq!((records, work), (table_records + 1, table_work));
            }
            if mode != "empty" {
                check_api!("definition", prepare_book_v2_definition_mixed_search, prepare_book_v2_definition_mixed_search_counted, initial, work_charge, &measured, 0);
                if mode == "multiple" {
                    check_api!("second definition", prepare_book_v2_definition_mixed_search, prepare_book_v2_definition_mixed_search_counted, initial, work_charge, &measured, 1);
                }
            }
            let mut records = 0;
            let mut work = 0;
            let prior = max_records + 1;
            assert_eq!(prepare_book_v2_definition_mixed_search_counted(&measured, usize::MAX, &limits, 0, prior, &mut records, &mut work).err().unwrap().kind, E::ReceiptMismatch);
            assert_eq!((records, work), (prior, 0));
            assert_eq!(prepare_book_v2_definition_table_demand_search_counted(&measured, usize::MAX, &limits, 0, prior, &mut records, &mut work).err().unwrap().kind, E::ReceiptMismatch);
            assert_eq!((records, work), (prior, 0));
        }).unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}
