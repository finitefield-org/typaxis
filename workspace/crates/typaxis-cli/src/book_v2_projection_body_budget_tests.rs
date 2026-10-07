use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_body_flow_counted,
    prepare_book_v2_table_measurements, prepare_book_v2_table_measurements_counted,
};
use typaxis_pagination::ProductionBodyPaginationErrorKind as P;

#[test]
fn book_v2_projection_constructor_budget_retains_body_and_measurement_records() {
    check(None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_projection_constructor_budget_retains_original_harano_records() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font));
}

fn check(font: Option<&[u8]>) {
    let maximum = 100_000_000;
    for mode in ["empty", "nested", "conflict", "spool"] {
        let root = Root::new();
        let limits = if mode == "spool" {
            let mut caps = limits().base().get().clone();
            // Admission needs less than 2,560 bytes; the three-table measurement
            // reserves a larger canonical buffer after its record reservation.
            caps.max_spool_bytes = 2560;
            M4EffectiveResourceLimits::new(
                ValidatedResourceLimits::new(caps).unwrap(),
                M4ResourceLimits {
                    max_font_subset_bytes: 2560,
                    ..M4ResourceLimits::default()
                },
            )
            .unwrap()
        } else {
            limits()
        };
        let (text, split) = if font.is_some() {
            ("本文の柱", 6)
        } else {
            ("LeftRight", 4)
        };
        let input = if mode == "empty" {
            input(&root, text, font, &limits)
        } else {
            let mut data = super::super::nested_tables::nested(text, split, "two-children");
            if mode == "conflict" {
                data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["classes"] =
                    json!(["conflict"]);
                let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
                rules.push(json!({"style_id":"conflict","selector":"table.conflict","source_order":rules.len(),"extends":null,"declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"other"}}]}));
                let master = data["page_masters"]["masters"][0]["master_id"].clone();
                let rules = data["page_masters"]["selection_rules"]
                    .as_array_mut()
                    .unwrap();
                rules.push(json!({"master_id":master,"named_page":"other","parity":"any","first":null,"source_order":rules.len()}));
            }
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
            prepare_book_v2_resources(
                body,
                &root.context(),
                &config_with_extension(
                    limits.base().get().clone(),
                    limits.extension().get().clone(),
                ),
                &limits,
            )
            .unwrap()
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_with_regions(
            &flow, &mut 0, maximum, 0, 0,
        )
        .unwrap();
        assert!(!plan.requires_width_reflow());
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
                let initial = command_source_record_charge(&flow)
                    + plan.record_charge() + lines.source_record_charge() + lines.retained_record_charge();
                let body = |prior, observed: &mut u64| {
                    prepare_book_v2_body_flow_counted(
                        lines.lines(),
                        None,
                        lines.footnotes(),
                        &limits,
                        prior,
                        observed,
                    )
                };
                let legacy = |prior| {
                    prepare_book_v2_body_flow(
                        lines.lines(),
                        None,
                        lines.footnotes(),
                        &limits,
                        prior,
                    )
                };
                let mut records = 0;
                let result = body(initial, &mut records);
                let full_body = result.unwrap();
                let body_records = full_body.record_charge();
                assert_eq!(records, body_records);
                assert_eq!(body_records, legacy(initial).unwrap().record_charge());
                let measured =
                    prepare_book_v2_table_measurements_counted(full_body, &limits, &mut records);
                if mode == "spool" {
                    let error = measured.err().unwrap();
                    assert_eq!(error.kind, P::SpoolLimit);
                    assert!(records > body_records);
                    assert_eq!(
                        error,
                        prepare_book_v2_table_measurements(legacy(initial).unwrap(), &limits)
                            .err()
                            .unwrap()
                    );
                    assert_driver_failure(&input, &limits, "tables", error, records);
                    return;
                }
                let measured = measured.unwrap();
                assert_eq!(records, measured.record_charge());
                assert_eq!(
                    measured.fingerprint(),
                    prepare_book_v2_table_measurements(legacy(initial).unwrap(), &limits)
                        .unwrap()
                        .fingerprint()
                );
                let maximum_records = limits.base().get().max_fragments;
                let body_extra = body_records - initial;
                assert!(body_extra > 1);
                let mut retained_partial = false;
                for remaining in 0..body_extra {
                    let prior = maximum_records - remaining;
                    let error = body(prior, &mut records).err().unwrap();
                    assert_eq!(error.kind, P::FragmentLimit);
                    assert_eq!(error, legacy(prior).err().unwrap());
                    assert!(records >= prior && records <= maximum_records);
                    retained_partial |= records > prior;
                }
                assert!(retained_partial);
                assert_eq!(
                    body(maximum_records - body_extra, &mut records)
                        .unwrap()
                        .record_charge(),
                    maximum_records
                );
                assert_eq!(records, maximum_records);
                let exact_prior = maximum_records - (measured.record_charge() - initial);
                let exact = body(exact_prior, &mut records).unwrap();
                let exact =
                    prepare_book_v2_table_measurements_counted(exact, &limits, &mut records)
                        .unwrap();
                assert_eq!(records, maximum_records);
                assert_eq!(exact.fingerprint(), measured.fingerprint());
                if measured.record_charge() > body_records {
                    let short = body(exact_prior + 1, &mut records).unwrap();
                    let before = records;
                    assert_eq!(
                        prepare_book_v2_table_measurements_counted(short, &limits, &mut records)
                            .err()
                            .unwrap()
                            .kind,
                        P::FragmentLimit
                    );
                    assert_eq!(records, before);
                }
                if mode == "conflict" {
                    // Named content is now admitted by body/measurement owners.
                    // The incompatible active parallel cells are rejected only
                    // after their real continuation positions are available.
                    let mut search = typaxis_pagination::book_v2::prepare_book_v2_table_body_search(
                        &measured, &limits, maximum, measured.record_charge(),
                    ).unwrap();
                    let mut passes = 0;
                    let error = search.select_stable_mixed_pages_counted(
                        limits.base().get().max_layout_passes, &mut passes,
                    ).err().unwrap();
                    assert_eq!(error.kind, P::PendingNamedPage);
                    assert_eq!(passes, 1);
                    assert!(search.record_charge() > measured.record_charge());
                    assert_driver_failure(&input, &limits, "page stability", error, search.record_charge());
                }
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}

fn assert_driver_failure(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    expected_stage: &str,
    cause: typaxis_pagination::ProductionBodyPaginationError,
    records: u64,
) {
    let mut budget = BookV2PdfConvergenceBudget::new(limits, 100_000_000);
    let error = with_budgeted_book_v2_pdf(input, limits, MODE, &mut budget, |_, _| {
        panic!("failed projection reached PDF")
    })
    .unwrap_err();
    let BookV2ConvergenceError::Stage { stage, source } = error else {
        panic!("{error:?}")
    };
    assert_eq!(stage, expected_stage);
    assert_eq!(
        *source
            .downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>()
            .unwrap(),
        cause
    );
    let retained = budget.observation();
    assert_eq!(retained.record_charge(), records);
    assert_eq!(retained.output_charge(), 0);
    assert_eq!(retained.page_passes(), u16::from(expected_stage == "page stability"));
    assert_eq!(retained.candidate_passes(), 0);
    assert!(with_budgeted_book_v2_pdf(input, limits, MODE, &mut budget, |_, _| ()).is_err());
    assert!(budget.observation().record_charge() >= retained.record_charge());
    assert!(budget.observation().work_steps() >= retained.work_steps());
}
