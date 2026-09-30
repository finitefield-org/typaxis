use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_pdf, BookV2ConvergenceError, BookV2PdfConvergenceBudget,
};
use typaxis_layout::book_v2::{
    compute_book_v2_native_math_counted as counted, prepare_book_v2_inline_items_counted,
    with_budgeted_book_v2_body_lines, with_converged_book_v2_body_lines, BookV2BodyLineBudget,
    BookV2NativeMathBudgetObservation as Observation,
};

#[test]
fn book_v2_native_budget_preserves_atomic_preflight_and_owned_line_contexts() {
    let root = Root::new();
    let limits = limits();
    let input = native_input(&root, native_data(), &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut observed = Observation::default();
    let native = counted(&bindings, input.resources(), &limits, 0, 0, &mut observed)
        .unwrap()
        .unwrap();
    let initial = observed;
    assert_eq!(native.layout_work(), initial.reserved_layout_units());
    assert_eq!(native.record_charge(), initial.record_charge());
    assert_eq!(native.spool_charge(), initial.spool_charge());
    assert_eq!(
        native.fingerprint(),
        compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
            .unwrap()
            .unwrap()
            .fingerprint()
    );
    assert_eq!(
        initial.reserved_layout_units(),
        native
            .receipts()
            .iter()
            .map(|r| r.computation().layout_work())
            .sum::<u64>()
    );
    let cap = limits.base().get();
    let record_prior = cap.max_fragments - initial.record_charge();
    let spool_prior = cap.max_spool_bytes - initial.spool_charge();
    let exact = counted(
        &bindings,
        input.resources(),
        &limits,
        record_prior,
        spool_prior,
        &mut observed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(exact.fingerprint(), native.fingerprint());
    assert_eq!(
        (observed.record_charge(), observed.spool_charge()),
        (cap.max_fragments, cap.max_spool_bytes)
    );
    for (records, spool, work, kept_records, kept_spool, cause) in [
        (
            record_prior + 1,
            7,
            initial.reserved_layout_units(),
            record_prior + 1,
            7,
            ProductionNativeMathComputationError::RecordLimit,
        ),
        (
            11,
            spool_prior + 1,
            initial.reserved_layout_units(),
            11 + initial.record_charge(),
            spool_prior + 1,
            ProductionNativeMathComputationError::SpoolLimit,
        ),
        (
            u64::MAX,
            0,
            0,
            u64::MAX,
            0,
            ProductionNativeMathComputationError::RecordLimit,
        ),
        (
            0,
            u64::MAX,
            0,
            0,
            u64::MAX,
            ProductionNativeMathComputationError::SpoolLimit,
        ),
        (
            u64::MAX,
            u64::MAX,
            0,
            u64::MAX,
            u64::MAX,
            ProductionNativeMathComputationError::RecordLimit,
        ),
    ] {
        assert_eq!(
            counted(
                &bindings,
                input.resources(),
                &limits,
                records,
                spool,
                &mut observed
            )
            .err()
            .unwrap(),
            cause
        );
        assert_eq!(
            compute_book_v2_native_math(&bindings, input.resources(), &limits, records, spool)
                .err()
                .unwrap(),
            cause
        );
        assert_eq!(observed.reserved_layout_units(), work);
        assert_eq!(
            (observed.record_charge(), observed.spool_charge()),
            (kept_records, kept_spool)
        );
    }
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let mut records = u64::MAX;
    let owned = prepare_book_v2_inline_items_counted(
        &flow,
        &shaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        &mut observed,
        &mut records,
    )
    .unwrap();
    assert_eq!(observed, initial);
    assert!(records > initial.record_charge());
    assert_eq!(
        owned.fingerprint(),
        prepare_book_v2_inline_items(
            &flow,
            &shaped,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal
        )
        .unwrap()
        .fingerprint()
    );
    let raw = |n| Length::from_raw(n).unwrap();
    let frame = typaxis_core::Rect::new(
        raw(500_000),
        raw(500_000),
        PositiveLength::new(raw(10_000_000)).unwrap(),
        PositiveLength::new(raw(10_000_000)).unwrap(),
    );
    let expected = with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        frame,
        1_000_000,
        |stable| {
            (
                stable.lines().fingerprint(),
                stable.candidate_steps(),
                stable.passes().len() as u16,
            )
        },
    )
    .unwrap();
    let mut exact = BookV2BodyLineBudget::new(expected.1, expected.2);
    let same = with_budgeted_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        frame,
        &mut exact,
        &mut observed,
        |stable| {
            (
                stable.lines().fingerprint(),
                stable.candidate_steps(),
                stable.passes().len() as u16,
            )
        },
    )
    .unwrap();
    assert_eq!(same, expected);
    assert_eq!(observed, initial);
    assert_eq!((exact.remaining_steps(), exact.remaining_passes()), (0, 0));
    let mut zero = BookV2BodyLineBudget::new(0, cap.max_line_reshape_passes);
    for _ in 0..2 {
        assert!(with_budgeted_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            frame,
            &mut zero,
            &mut observed,
            |_| panic!("zero work must not emit lines"),
        )
        .is_err());
        assert_eq!(observed, initial);
        assert_eq!(zero.candidate_steps(), 0);
        assert_eq!(zero.reshape_passes(), 0);
        assert!(zero.record_charge() >= initial.record_charge());
    }
    // Native-free and foreign-resource checks precede any native reservation.
    let other_root = Root::new();
    let empty = prepared(&other_root, source_data("Result"), b"Result", &limits);
    let empty_policy = prepare_book_v2_resource_policy(empty.body(), &limits).unwrap();
    let empty_bindings = bind_book_v2_vectors(&empty_policy, empty.resources(), &limits).unwrap();
    assert!(counted(
        &empty_bindings,
        empty.resources(),
        &limits,
        u64::MAX,
        u64::MAX,
        &mut observed
    )
    .unwrap()
    .is_none());
    assert_eq!(observed, Observation::new(u64::MAX, u64::MAX));
    assert!(counted(&bindings, empty.resources(), &limits, 29, 31, &mut observed).is_err());
    assert_eq!(observed, Observation::new(29, 31));
    for mode in ["work", "records", "spool"] {
        let mut caps = limits.base().get().clone();
        let mut extension = limits.extension().get().clone();
        match mode {
            "work" => extension.max_math_layout_units = initial.reserved_layout_units() - 1,
            "records" => caps.max_fragments = initial.record_charge() - 1,
            "spool" => caps.max_spool_bytes = initial.spool_charge() - 1,
            _ => unreachable!(),
        }
        extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(caps.max_spool_bytes);
        let bounded =
            M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(caps).unwrap(), extension)
                .unwrap();
        let root = Root::new();
        let input = native_input(&root, native_data(), &bounded);
        let policy = prepare_book_v2_resource_policy(input.body(), &bounded).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &bounded).unwrap();
        let cause = counted(&bindings, input.resources(), &bounded, 0, 0, &mut observed)
            .err()
            .unwrap();
        let reserved = observed;
        assert_eq!(
            cause,
            compute_book_v2_native_math(&bindings, input.resources(), &bounded, 0, 0)
                .err()
                .unwrap()
        );
        assert_eq!(
            reserved.reserved_layout_units(),
            if mode == "work" {
                0
            } else {
                initial.reserved_layout_units()
            }
        );
        assert_eq!(
            reserved.record_charge(),
            if mode == "spool" {
                initial.record_charge()
            } else {
                0
            }
        );
        assert_eq!(reserved.spool_charge(), 0);
        check_driver(&input, &bounded, &cause, reserved);
    }
}

#[test]
fn book_v2_native_budget_retains_font_failures_and_command_retry_history() {
    check_missing_font(FONT, "sfnt-truetype-glyf");
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_native_budget_retains_original_harano_math_font_rejection() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_missing_font(&font, "sfnt-cff1");
}

fn check_missing_font(font: &[u8], media: &str) {
    let limits = limits();
    for later in [false, true] {
        let root = Root::new();
        let input = failing_font_input(&root, &limits, font, media, later);
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let mut observed = Observation::default();
        let cause = counted(&bindings, input.resources(), &limits, 0, 0, &mut observed)
            .err()
            .unwrap();
        let initial = observed;
        assert_eq!(
            cause,
            StagingMathLayoutError::InvalidMathFont(NodeId::new(if later { 6 } else { 4 })).into()
        );
        assert_eq!(
            cause,
            compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
                .err()
                .unwrap()
        );
        assert!(
            initial.record_charge() > 0
                && initial.spool_charge() > 0
                && initial.reserved_layout_units() > 0
        );
        check_driver(&input, &limits, &cause, initial);
        let failure = counted(
            &bindings,
            input.resources(),
            &limits,
            initial.record_charge(),
            initial.spool_charge(),
            &mut observed,
        )
        .err()
        .unwrap();
        assert_eq!(failure, cause);
        assert_eq!(observed.record_charge(), 2 * initial.record_charge());
        assert_eq!(observed.spool_charge(), 2 * initial.spool_charge());
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let shaped = shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            bindings.epoch(),
            None,
        )
        .unwrap();
        let mut records = 0;
        let failure = prepare_book_v2_inline_items_counted(
            &flow,
            &shaped,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            &mut observed,
            &mut records,
        )
        .err()
        .unwrap();
        assert_eq!(
            failure.kind,
            typaxis_layout::ProductionInlinePreparationErrorKind::NativeMath(cause.clone())
        );
        assert_eq!(observed, initial);
        assert_eq!(records, initial.record_charge());
        let raw = |n| Length::from_raw(n).unwrap();
        let frame = typaxis_core::Rect::new(
            raw(0),
            raw(0),
            PositiveLength::new(raw(10_000_000)).unwrap(),
            PositiveLength::new(raw(10_000_000)).unwrap(),
        );
        let mut line_budget =
            BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
        assert!(with_budgeted_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            frame,
            &mut line_budget,
            &mut observed,
            |_| panic!("failed native construction emitted lines"),
        )
        .is_err());
        assert_eq!(observed, initial);
        assert_eq!(line_budget.record_charge(), initial.record_charge());
        assert_eq!(
            (line_budget.candidate_steps(), line_budget.reshape_passes()),
            (0, 0)
        );
        // Exact remaining record/spool room for two failed computations, but
        // only one work reservation fits. Further retries may not reset history.
        let mut caps = limits.base().get().clone();
        caps.max_fragments = 2 * initial.record_charge();
        caps.max_spool_bytes = 2 * initial.spool_charge();
        let mut extension = limits.extension().get().clone();
        extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(caps.max_spool_bytes);
        let bounded =
            M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(caps).unwrap(), extension)
                .unwrap();
        let retry_root = Root::new();
        let retry_input = failing_font_input(&retry_root, &bounded, font, media, later);
        let mut command =
            BookV2PdfConvergenceBudget::new(&bounded, 2 * initial.reserved_layout_units() - 1);
        for attempt in 1..=3 {
            let failure = with_budgeted_book_v2_pdf(
                &retry_input,
                &bounded,
                JapaneseLineBreakMode::Normal,
                &mut command,
                |_, _| panic!("failed font emitted PDF"),
            )
            .err()
            .unwrap();
            assert_native_error(failure, &cause);
            let history = command.observation();
            assert_eq!(history.work_steps(), initial.reserved_layout_units());
            assert_eq!(
                history.record_charge(),
                initial.record_charge() * attempt.min(2)
            );
            assert_eq!(
                history.spool_charge(),
                initial.spool_charge() * attempt.min(2)
            );
            assert_eq!(
                (
                    history.output_charge(),
                    history.line_reshape_passes(),
                    history.page_passes(),
                    history.candidate_passes()
                ),
                (0, 0, 0, 0)
            );
        }
    }
}

fn failing_font_input(
    root: &Root,
    limits: &M4EffectiveResourceLimits,
    font: &[u8],
    media: &str,
    later: bool,
) -> PreparedBookV2Resources {
    let mut data = native_data();
    let face = if later { 1 } else { 0 };
    if later {
        data["resources"]["font_faces"][1] = data["resources"]["font_faces"][0].clone();
        data["resources"]["font_faces"][1]["font_face_id"] = 1.into();
        data["resources"]["font_faces"][1]["family"] = "BadMath".into();
        data["resources"]["font_faces"][1]["uri"] = "bad.bin".into();
        for declaration in data["style_sheet"]["rules"][3]["declarations"]
            .as_array_mut()
            .unwrap()
        {
            if declaration["name"] == "font_family" {
                declaration["value"]["families"] = json!(["BadMath"]);
            }
        }
    }
    data["resources"]["font_faces"][face]["expected_sha256"] = typaxis_core::sha256(font)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    data["resources"]["font_faces"][face]["media_type"] = media.into();
    let body = body_with_source(root, data, NATIVE_SOURCE, limits);
    fs::write(
        root.0.join("body.bin"),
        if later { NATIVE_BODY } else { font },
    )
    .unwrap();
    if later {
        fs::write(root.0.join("bad.bin"), font).unwrap();
    }
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        limits,
    )
    .unwrap()
}

fn check_driver(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    cause: &ProductionNativeMathComputationError,
    native: Observation,
) {
    for maximum in [
        100_000_000,
        native.reserved_layout_units().saturating_sub(1),
    ] {
        let mut command = BookV2PdfConvergenceBudget::new(limits, maximum);
        for attempt in 1..=2 {
            let failure = with_budgeted_book_v2_pdf(
                input,
                limits,
                JapaneseLineBreakMode::Normal,
                &mut command,
                |_, _| panic!("failed native emitted PDF"),
            )
            .err()
            .unwrap();
            assert_native_error(failure, cause);
            let kept = command.observation();
            assert_eq!(
                kept.work_steps(),
                if maximum == 100_000_000 {
                    attempt * native.reserved_layout_units()
                } else {
                    0
                }
            );
            assert_eq!(
                kept.record_charge(),
                if attempt * native.record_charge() <= limits.base().get().max_fragments {
                    attempt * native.record_charge()
                } else {
                    native.record_charge()
                }
            );
            assert_eq!(
                kept.spool_charge(),
                if attempt * native.spool_charge() <= limits.base().get().max_spool_bytes {
                    attempt * native.spool_charge()
                } else {
                    native.spool_charge()
                }
            );
            assert_eq!(
                (
                    kept.output_charge(),
                    kept.line_reshape_passes(),
                    kept.page_passes(),
                    kept.candidate_passes()
                ),
                (0, 0, 0, 0)
            );
        }
    }
}

fn assert_native_error(
    error: BookV2ConvergenceError,
    cause: &ProductionNativeMathComputationError,
) {
    let BookV2ConvergenceError::Stage { stage, source } = error else {
        panic!("native error precedence lost");
    };
    assert_eq!(stage, "native math");
    assert_eq!(
        source.downcast_ref::<ProductionNativeMathComputationError>(),
        Some(cause)
    );
}
