use super::*;
use crate::book_v2_resources::converged_pdf::{
    prepare_reserved_book_v2_navigation, with_reserved_book_v2_native_math,
};
use crate::book_v2_resources::BookV2PdfConvergenceObservation;
use std::cell::Cell;
use typaxis_layout::book_v2::{
    compute_preflighted_book_v2_native_math as execute,
    preflight_book_v2_native_math_counted as preflight,
};

#[test]
fn book_v2_native_command_preflight_seals_inputs_and_preserves_receipts() {
    let root = Root::new();
    let limits = limits();
    let input = native_input(&root, native_data(), &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let original = compute_book_v2_native_math(&bindings, input.resources(), &limits, 11, 13)
        .unwrap()
        .unwrap();
    let mut observed = Observation::default();
    let plan = preflight(&bindings, input.resources(), &limits, 11, 13, &mut observed)
        .unwrap()
        .unwrap();
    assert_eq!(plan.reserved_layout_units(), original.layout_work());
    assert_eq!(plan.record_charge(), original.record_charge());
    assert_eq!(plan.spool_charge(), original.spool_charge());
    assert_eq!(
        observed.reserved_layout_units(),
        plan.reserved_layout_units()
    );
    assert_eq!(observed.record_charge(), plan.record_charge());
    assert_eq!(observed.spool_charge(), plan.spool_charge());
    let computed = execute(plan, &limits).unwrap();
    assert_eq!(computed.fingerprint(), original.fingerprint());
    assert_eq!(computed.receipts().len(), original.receipts().len());
    for (actual, expected) in computed.receipts().iter().zip(original.receipts()) {
        assert_eq!(actual.fingerprint(), expected.fingerprint());
        assert_eq!(
            actual.computation().fingerprint(),
            expected.computation().fingerprint()
        );
        assert_eq!(
            actual.computation().paints(),
            expected.computation().paints()
        );
    }
    computed
        .verify(&bindings, input.resources(), &limits)
        .unwrap();
    let mut caps = limits.base().get().clone();
    caps.max_fragments -= 1;
    let foreign_limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(caps).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    let plan = preflight(&bindings, input.resources(), &limits, 19, 23, &mut observed)
        .unwrap()
        .unwrap();
    assert_eq!(
        execute(plan, &foreign_limits).err().unwrap(),
        StagingMathLayoutError::ReceiptMismatch.into()
    );
    let other_root = Root::new();
    let other = native_input(&other_root, native_data(), &limits);
    let other_policy = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    let other_bindings = bind_book_v2_vectors(&other_policy, other.resources(), &limits).unwrap();
    assert!(computed
        .verify(&other_bindings, other.resources(), &limits)
        .is_err());
    assert_eq!(
        preflight(&bindings, other.resources(), &limits, 29, 31, &mut observed)
            .err()
            .unwrap(),
        StagingMathLayoutError::ReceiptMismatch.into()
    );
    assert_eq!(observed, Observation::new(29, 31));
    let empty_root = Root::new();
    let empty = prepared(&empty_root, source_data("Result"), b"Result", &limits);
    let empty_policy = prepare_book_v2_resource_policy(empty.body(), &limits).unwrap();
    let empty_bindings = bind_book_v2_vectors(&empty_policy, empty.resources(), &limits).unwrap();
    assert!(preflight(
        &empty_bindings,
        empty.resources(),
        &limits,
        u64::MAX,
        u64::MAX,
        &mut observed,
    )
    .unwrap()
    .is_none());
    assert_eq!(observed, Observation::new(u64::MAX, u64::MAX));
    let mut history = BookV2PdfConvergenceObservation::default();
    let native = with_reserved_book_v2_native_math(
        &empty_bindings,
        empty.resources(),
        &limits,
        0,
        &mut history,
        |plan| {
            assert!(plan.is_none());
            plan.map(|plan| execute(plan, &limits)).transpose()
        },
    )
    .unwrap();
    assert!(native.is_none());
    assert_eq!(history, BookV2PdfConvergenceObservation::default());
}

#[test]
fn book_v2_native_command_preflight_stops_construction_on_each_command_bound() {
    check_command_bounds(FONT, "sfnt-truetype-glyf");
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_native_command_preflight_stops_original_harano_on_command_bounds() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_command_bounds(&font, "sfnt-cff1");
}

fn check_command_bounds(font: &[u8], media: &str) {
    let limits = limits();
    for later in [false, true] {
        let root = Root::new();
        let input = failing_font_input(&root, &limits, font, media, later);
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let mut observed = Observation::default();
        // Missing MATH is accepted by preflight and rejected only by execution.
        let plan = preflight(&bindings, input.resources(), &limits, 0, 0, &mut observed)
            .unwrap()
            .unwrap();
        let cause = execute(plan, &limits).err().unwrap();
        assert_eq!(
            cause,
            StagingMathLayoutError::InvalidMathFont(NodeId::new(if later { 6 } else { 4 })).into()
        );
        let work = observed.reserved_layout_units();
        let records = observed.record_charge();
        let navigation_records = prepare_book_v2_navigation(input.body().styled())
            .unwrap().source_record_charge();
        let spool = observed.spool_charge();
        for mode in [
            "fresh work",
            "work",
            "records",
            "spool",
            "records and spool",
            "all",
        ] {
            let mut caps = limits.base().get().clone();
            let mut extension = limits.extension().get().clone();
            let maximum_work = match mode {
                "fresh work" => work - 1,
                "work" | "all" => 2 * work - 1,
                _ => 100_000_000,
            };
            if matches!(mode, "records" | "records and spool" | "all") {
                caps.max_fragments = 2 * (navigation_records + records) - 1;
            }
            if matches!(mode, "spool" | "records and spool" | "all") {
                caps.max_spool_bytes = 2 * spool - 1;
            }
            extension.max_font_subset_bytes =
                extension.max_font_subset_bytes.min(caps.max_spool_bytes);
            let bounded = M4EffectiveResourceLimits::new(
                ValidatedResourceLimits::new(caps).unwrap(),
                extension,
            )
            .unwrap();
            let bounded_root = Root::new();
            let input = failing_font_input(&bounded_root, &bounded, font, media, later);
            let policy = prepare_book_v2_resource_policy(input.body(), &bounded).unwrap();
            let bindings = bind_book_v2_vectors(&policy, input.resources(), &bounded).unwrap();
            let entered = Cell::new(0);
            let mut history = BookV2PdfConvergenceObservation::default();
            let mut command = BookV2PdfConvergenceBudget::new(&bounded, maximum_work);
            for attempt in 1..=3 {
                let _navigation = prepare_reserved_book_v2_navigation(
                    input.body().styled(), &bounded, &mut history,
                ).unwrap();
                let local = with_reserved_book_v2_native_math(
                    &bindings,
                    input.resources(),
                    &bounded,
                    maximum_work,
                    &mut history,
                    |plan| {
                        entered.set(entered.get() + 1);
                        plan.map(|plan| execute(plan, &bounded)).transpose()
                    },
                )
                .err()
                .unwrap();
                let complete = with_budgeted_book_v2_pdf(
                    &input,
                    &bounded,
                    JapaneseLineBreakMode::Normal,
                    &mut command,
                    |_, _| panic!("invalid font must not emit a PDF"),
                )
                .err()
                .unwrap();
                if attempt == 1 && mode != "fresh work" {
                    assert_native_error(local, &cause);
                    assert_native_error(complete, &cause);
                } else {
                    let limit = match mode {
                        "records" | "records and spool" => "records",
                        "spool" => "spool",
                        _ => "work",
                    };
                    assert_command_limit(local, limit);
                    assert_command_limit(complete, limit);
                }
                // The exact continuation used by the driver never runs after
                // a command reservation is rejected, even on later retries.
                assert_eq!(entered.get(), if mode == "fresh work" { 0 } else { 1 });
                assert_eq!(command.observation(), history);
                assert_eq!(
                    history.work_steps(),
                    match mode {
                        "fresh work" => 0,
                        "work" | "all" => work,
                        _ => attempt * work,
                    }
                );
                assert_eq!(
                    history.record_charge(),
                    if matches!(mode, "records" | "records and spool" | "all") {
                        records + attempt * navigation_records
                    } else {
                        attempt * (navigation_records + records)
                    }
                );
                assert_eq!(
                    history.spool_charge(),
                    if matches!(mode, "spool" | "records and spool" | "all") {
                        spool
                    } else {
                        attempt * spool
                    }
                );
                assert_eq!(
                    (
                        history.output_charge(),
                        history.candidate_passes(),
                        history.line_reshape_passes(),
                        history.page_passes()
                    ),
                    (0, 0, 0, 0)
                );
            }
        }
    }
}
