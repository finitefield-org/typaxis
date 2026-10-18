//! Real command controls after adopting the observed source, physical,
//! running-region, and PDF route. Original kernels remain geometry oracles.
use super::*;
use driver::converged_column_pdf::{
    with_converged_column_pdf_observer, ColumnAttemptObserver, ColumnPdfAttempt,
};
use driver::{BookV2ColumnPdfBundle, BookV2ColumnPdfConvergenceBudget};

fn running(mut d: Value) -> Value {
    let template = data(true)["page_masters"]["masters"][0].clone();
    let master = &mut d["page_masters"]["masters"][0];
    for field in ["header", "header_content", "footer"] {
        master[field] = template[field].clone();
    }
    master["footer_content"] = template["header_content"].clone();
    let mut next = 0;
    renumber(&mut d["document"], &mut next);
    for m in d["page_masters"]["masters"].as_array_mut().unwrap() {
        renumber(&mut m["header_content"], &mut next);
        renumber(&mut m["footer_content"], &mut next);
    }
    d
}

#[test]
fn book_v2_column_observed_pdf_native_plain_and_running_regions_match_original_pdf() {
    for native in [false, true] {
        for rich in [false, true] {
            let root = Root::new();
            let limits = caps();
            let input = native_input(
                &root,
                running(variant_data(native, rich)),
                &limits,
                MATH_FONT,
            );
            let mut legacy = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            let expected = driver::pdf_driver::with_budgeted_book_v2_column_pdf(
                &input,
                &limits,
                MODE,
                &mut legacy,
                4,
                |roots, _, _| {
                    (
                        roots.pdf().bytes().to_vec(),
                        roots.display().fingerprint(),
                        roots.pdf().fingerprint(),
                    )
                },
            )
            .unwrap();
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limits, MAXIMUM);
            let (actual, observed) = driver::with_budgeted_converged_book_v2_column_pdf(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |roots, observed| {
                    assert!(!roots.display().page_regions().is_empty());
                    assert_eq!(
                        roots
                            .display()
                            .source()
                            .original_graph()
                            .lines()
                            .prepared()
                            .native_math()
                            .is_some(),
                        native
                    );
                    (
                        (
                            roots.pdf().bytes().to_vec(),
                            roots.display().fingerprint(),
                            roots.pdf().fingerprint(),
                        ),
                        observed,
                    )
                },
            )
            .unwrap();
            assert_eq!(actual, expected, "native={native} rich={rich}");
            assert_eq!(observed, budget.observation());
            assert_eq!(observed.candidates, 1);
            assert!(observed.spool > legacy.observation().spool);
        }
    }
}

#[derive(Default)]
struct NativeAttempts {
    addresses: Vec<usize>,
    values: Vec<Vec<u32>>,
    after: Vec<BookV2PdfConvergenceObservation>,
}

#[test]
fn book_v2_column_observed_pdf_native_running_exact_and_short_work_retain_the_complete_prefix() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, running(variant_data(true, true)), &limits, MATH_FONT);
    let mut baseline = BookV2ColumnPdfConvergenceBudget::new(&limits, MAXIMUM);
    let expected = driver::with_budgeted_converged_book_v2_column_pdf(
        &input,
        &limits,
        MODE,
        &mut baseline,
        4,
        |roots, _| roots.pdf().bytes().to_vec(),
    )
    .unwrap();
    let complete = baseline.observation();
    for maximum in [complete.work - 1, complete.work] {
        let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limits, maximum);
        let mut called = false;
        let result = driver::with_budgeted_converged_book_v2_column_pdf(
            &input,
            &limits,
            MODE,
            &mut budget,
            4,
            |roots, _| {
                called = true;
                roots.pdf().bytes().to_vec()
            },
        );
        let actual = budget.observation();
        assert_eq!(result.is_ok(), maximum == complete.work, "{result:?}");
        assert_eq!(called, maximum == complete.work);
        assert_eq!(actual.records, complete.records);
        assert_eq!(actual.spool, complete.spool);
        assert_eq!(actual.output, complete.output);
        assert_eq!(actual.line_passes, complete.line_passes);
        assert_eq!(actual.page_passes, complete.page_passes);
        assert_eq!(actual.work, maximum);
        if let Ok(bytes) = result {
            assert_eq!(bytes, expected);
        } else {
            let error = driver::with_budgeted_converged_book_v2_column_pdf::<()>(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |_, _| panic!("exhausted retry delivered PDF"),
            );
            assert!(error.is_err());
            let after = budget.observation();
            assert!(
                after.records >= actual.records
                    && after.spool >= actual.spool
                    && after.work >= actual.work
            );
            assert_eq!(after.output, actual.output);
        }
    }
}

#[test]
fn book_v2_column_observed_pdf_native_owner_spans_actual_page_label_retries_and_opaque_error() {
    let root = Root::new();
    let limits = caps();
    let mut d = variant_data(true, true);
    let span = d["document"]["blocks"][0]["blocks"][2]["span"].clone();
    let mut reference = page();
    reference["span"] = span.clone();
    d["document"]["blocks"][0]["blocks"][2]["children"]
        .as_array_mut()
        .unwrap()
        .push(reference);
    let blocks = d["document"]["blocks"][0]["blocks"].as_array_mut().unwrap();
    blocks
        .extend((0..9).map(|_| json!({"kind":"page_break","node_id":0,"classes":[],"span":span})));
    blocks.push(json!({"kind":"heading","node_id":0,"classes":[],"span":span,"level":1,"anchor_id":"page-target",
        "children":[{"kind":"text","node_id":0,"span":span,"text_span":{"text_id":0,"start_byte":NATIVE_SOURCE.len(),"end_byte":NATIVE_SOURCE.len()+7}}]}));
    let input = native_input(&root, running(d), &limits, MATH_FONT);
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limits, MAXIMUM);
    let mut observer = NativeAttempts::default();
    let delivered = with_converged_column_pdf_observer(
        &input,
        &limits,
        MODE,
        &mut budget,
        4,
        |roots, _| {
            assert!(roots.pdf().page_reference_labels_match());
            Err::<(), _>("opaque application result")
        },
        &mut observer,
    )
    .unwrap();
    assert_eq!(delivered, Err("opaque application result"));
    assert!(observer.values.len() >= 3);
    assert_eq!(observer.values[0], [1]);
    assert!(observer.values[1][0] >= 10);
    assert!(observer
        .addresses
        .iter()
        .all(|p| *p == observer.addresses[0]));
    assert_eq!(observer.after.last(), Some(&budget.observation()));
    let before = budget.observation();
    driver::with_budgeted_converged_book_v2_column_pdf(
        &input,
        &limits,
        MODE,
        &mut budget,
        4,
        |_, _| (),
    )
    .unwrap();
    let after = budget.observation();
    assert!(
        after.records > before.records && after.spool > before.spool && after.work > before.work
    );
    assert!(after.page_passes > before.page_passes && after.line_passes > before.line_passes);
}

#[test]
#[ignore = "requires TYPAXIS_COLUMN_OBSERVED_VMB_JOBS with original-font VMB exports"]
fn book_v2_column_observed_pdf_real_vmb_exports_match_pdf_and_reuse_original_cff_session() {
    let index: Value = serde_json::from_slice(
        &fs::read(std::env::var("TYPAXIS_COLUMN_OBSERVED_VMB_JOBS").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(index.as_object().unwrap().len(), 10);
    for (name, record) in index.as_object().unwrap() {
        let limits = caps();
        let (input, path, original) = prepared_saved_export(record, &limits);
        let maximum = 1_000_000_000;
        let mut legacy = driver::BookV2ColumnPageBudget::new(&limits, maximum);
        let expected = driver::pdf_driver::with_budgeted_book_v2_column_pdf(
            &input,
            &limits,
            MODE,
            &mut legacy,
            4,
            |roots, _, _| roots.pdf().bytes().to_vec(),
        )
        .unwrap();
        let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limits, maximum);
        for retry in 0..2 {
            let before = budget.cff_program_statistics();
            let actual = driver::with_budgeted_converged_book_v2_column_pdf(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |roots, observed| {
                    assert!(observed.records > 0);
                    roots.pdf().bytes().to_vec()
                },
            )
            .unwrap();
            assert_eq!(actual, expected, "{name}, retry {retry}");
            let after = budget.cff_program_statistics();
            assert!(after.operations_used > 0 && after.cached_glyphs > 0);
            if retry == 1 {
                assert_eq!(after.operations_used, before.operations_used);
                assert_eq!(after.cached_glyphs, before.cached_glyphs);
                assert_eq!(after.outline_segments_used, before.outline_segments_used);
            }
        }
        assert_eq!(fs::read(path).unwrap(), original);
        eprintln!(
            "observed PDF original-font VMB {name}: {:?}",
            budget.cff_program_statistics()
        );
    }
}

impl ColumnAttemptObserver for NativeAttempts {
    fn observe<
        'event,
        'pdf,
        'nav,
        'w,
        'l,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >(
        &mut self,
        roots: &'event BookV2ColumnPdfBundle<
            'pdf,
            'nav,
            'w,
            'l,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        attempt: ColumnPdfAttempt<'event>,
    ) {
        let display = roots.display();
        let terminals = display.source();
        let prepared = terminals.original_graph().lines().prepared();
        let native = prepared
            .native_math()
            .expect("real inline and display native fractions");
        assert!(terminals.terminals().len() >= 2);
        assert_eq!(
            display.page_regions().len(),
            terminals.source().geometry().pages().len() * 2
        );
        assert_eq!(
            prepared.source_flow().page_reference_values().unwrap(),
            attempt.values
        );
        assert!(std::ptr::eq(roots.pdf().bytes(), attempt.pdf_bytes));
        self.addresses.push(native as *const _ as usize);
        self.values
            .push(attempt.values.iter().map(|(_, page)| *page).collect());
    }
    fn after_attempt(
        &mut self,
        observed: BookV2PdfConvergenceObservation,
        _: typaxis_resources::book_v2::BookV2CffProgramStatistics,
        ok: bool,
    ) {
        assert!(ok);
        self.after.push(observed);
    }
}
