fn production_context_font_fixture(text: &str) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&[text], "Body")).unwrap();
    value["resources"]["font_faces"][0]["uri"] = "body-context.ttf".into();
    value["resources"]["font_faces"][0]["expected_sha256"] =
        "abd89ef5ab470c02abf50090b7063114eae384387c36f339ae2cb85bfd7500d0".into();
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn production_final_line_reshape_changes_contextual_glyphs_at_actual_breaks() {
    with_production_inline_context(
        &production_context_font_fixture("A B"),
        &config(),
        |prepared, package, profile, limits, admitted, bindings| {
            let flow = prepared.source_flow();
            let initial_shape = typaxis_shaping::shape_production_authored_text(
                package,
                flow.navigation(),
                flow,
                admitted,
                limits,
                bindings.epoch().fingerprint(),
            )
            .unwrap();
            let initial_gids = initial_shape.paragraphs()[0]
                .runs()
                .iter()
                .flat_map(|r| &r.glyph_run().glyphs)
                .map(|g| g.original_gid.get())
                .collect::<Vec<_>>();
            assert_eq!(initial_gids, [4, 1, 3]); // A -> C before space B.
            let width = PositiveLength::new(Length::from_raw(1_000_000).unwrap()).unwrap();
            let lines = typaxis_layout::layout_production_inline_lines(prepared, &[width], 100_000)
                .unwrap();
            assert_eq!(lines.paragraphs()[0].lines().len(), 2);
            let contexts = typaxis_layout::production_selected_line_contexts(&lines).unwrap();
            assert_eq!(contexts.source_fingerprint(), lines.fingerprint());
            assert_eq!(contexts.record_charge(), lines.output_records() + 4);
            assert_eq!(contexts.paragraphs()[0].ends(), [2, 3]);
            let inputs = contexts
                .paragraphs()
                .iter()
                .map(|p| typaxis_shaping::ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                })
                .collect::<Vec<_>>();
            let shaped = typaxis_shaping::reshape_production_authored_text(
                package,
                flow.navigation(),
                flow,
                admitted,
                limits,
                bindings.epoch().fingerprint(),
                &inputs,
            )
            .unwrap();
            assert!(shaped.line_context_fingerprint().is_some());
            assert_ne!(shaped.fingerprint(), initial_shape.fingerprint());
            let glyphs = shaped.paragraphs()[0]
                .runs()
                .iter()
                .flat_map(|r| &r.glyph_run().glyphs)
                .map(|g| g.original_gid.get())
                .collect::<Vec<_>>();
            assert_eq!(glyphs, [2, 1, 3]); // No B context on the first selected line.
            let final_prepared = typaxis_layout::prepare_production_inline_items(
                package,
                flow.navigation(),
                profile,
                limits,
                admitted,
                flow,
                &shaped,
                bindings,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
            )
            .unwrap();
            let final_lines =
                typaxis_layout::layout_production_inline_lines(&final_prepared, &[width], 100_000)
                    .unwrap();
            let again = typaxis_layout::production_selected_line_contexts(&final_lines).unwrap();
            assert_eq!(
                again.paragraphs()[0].ends(),
                contexts.paragraphs()[0].ends()
            );
            let text = final_lines.paragraphs()[0]
                .lines()
                .iter()
                .flat_map(|l| l.items())
                .map(|i| match i {
                    typaxis_layout::ProductionPlacedInline::Text(t) => t.utf8(),
                    _ => panic!(),
                })
                .collect::<String>();
            assert_eq!(text, "A B");
        },
    );
}

#[test]
fn production_final_line_reshape_rejects_incomplete_foreign_and_invalid_boundaries() {
    with_production_inline_context(
        &production_context_font_fixture("A B"),
        &config(),
        |prepared, package, _, limits, admitted, bindings| {
            let flow = prepared.source_flow();
            let owner = flow.paragraphs()[0].owner();
            for (candidate, ends) in [
                (owner, vec![2]),
                (owner, vec![4]),
                (owner, vec![2, 1, 3]),
                (NodeId::new(999), vec![3]),
            ] {
                let input = [typaxis_shaping::ProductionParagraphLineContext {
                    owner: candidate,
                    ends: &ends,
                }];
                let failure = typaxis_shaping::reshape_production_authored_text(
                    package,
                    flow.navigation(),
                    flow,
                    admitted,
                    limits,
                    bindings.epoch().fingerprint(),
                    &input,
                )
                .err()
                .unwrap();
                assert_eq!(
                    failure.kind,
                    typaxis_shaping::ProductionTextShapeErrorKind::InvalidLineContext
                );
            }
        },
    );
}

#[test]
fn production_final_line_reshape_feedback_bounds_actual_width_oscillation() {
    let cfg = config_with_limits(ResourceLimits {
        max_line_reshape_passes: 2,
        ..ResourceLimits::default()
    });
    with_production_inline_context(
        &production_context_font_fixture("A B"),
        &cfg,
        |prepared, package, profile, limits, admitted, bindings| {
            let flow = prepared.source_flow();
            let width = PositiveLength::new(Length::from_raw(1_300_000).unwrap()).unwrap();
            let initial =
                typaxis_layout::layout_production_inline_lines(prepared, &[width], 100_000)
                    .unwrap();
            assert_eq!(initial.paragraphs()[0].lines().len(), 2);
            let state = |lines: &typaxis_layout::ProductionInlineLineLayout<'_, '_>| {
                let hex = lines
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                typaxis_linebreak::LineLayoutStateFingerprint::from_canonical_bytes(
                    format!("{{\"selected_line_layout_fingerprint\":\"{hex}\"}}").as_bytes(),
                )
                .unwrap()
            };
            let mut feedback = typaxis_linebreak::LineReshapeFeedback::new(state(&initial));
            let mut context = typaxis_linebreak::LineLayoutContext::from_limits(limits.base());
            let mut budget = context.take_budget().unwrap();
            let mut ends = typaxis_layout::production_selected_line_contexts(&initial)
                .unwrap()
                .paragraphs()[0]
                .ends()
                .to_vec();
            for pass in 1..=2 {
                let permit = feedback.begin_pass(&mut budget).unwrap();
                let inputs = [typaxis_shaping::ProductionParagraphLineContext {
                    owner: flow.paragraphs()[0].owner(),
                    ends: &ends,
                }];
                let shaped = typaxis_shaping::reshape_production_authored_text(
                    package,
                    flow.navigation(),
                    flow,
                    admitted,
                    limits,
                    bindings.epoch().fingerprint(),
                    &inputs,
                )
                .unwrap();
                let items = typaxis_layout::prepare_production_inline_items(
                    package,
                    flow.navigation(),
                    profile,
                    limits,
                    admitted,
                    flow,
                    &shaped,
                    bindings,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                )
                .unwrap();
                let lines =
                    typaxis_layout::layout_production_inline_lines(&items, &[width], 100_000)
                        .unwrap();
                // C is wider than A: clipped context fits one line, restored context needs two.
                assert_eq!(lines.paragraphs()[0].lines().len(), pass);
                ends = typaxis_layout::production_selected_line_contexts(&lines)
                    .unwrap()
                    .paragraphs()[0]
                    .ends()
                    .to_vec();
                let result = permit.complete(state(&lines));
                if pass == 1 {
                    assert_eq!(
                        result.unwrap(),
                        typaxis_linebreak::LineReshapeObservation::RebreakRequired
                    );
                } else {
                    assert_eq!(
                        result.unwrap_err(),
                        typaxis_linebreak::BreakError::IterationLimit
                    );
                }
            }
            assert_eq!(feedback.records().len(), 2);
            assert!(feedback.records().iter().all(|r| !r.is_stable()));
            assert_eq!(budget.remaining_reshape_passes(), 0);
            assert_eq!(
                feedback.begin_pass(&mut budget).err().unwrap(),
                typaxis_linebreak::BreakError::ReshapeTerminal
            );
        },
    );
}

#[test]
fn production_final_line_reshape_rejects_a_grapheme_split_before_font_shaping() {
    let cfg = config();
    let (package, navigation, limits, admitted) =
        production_text_fixture(&production_context_font_fixture("A\u{301}"), &cfg);
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let input = [typaxis_shaping::ProductionParagraphLineContext {
        owner: flow.paragraphs()[0].owner(),
        ends: &[1, 3],
    }];
    let failure = typaxis_shaping::reshape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        sha256(b"context-boundary-validation"),
        &input,
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.kind,
        typaxis_shaping::ProductionTextShapeErrorKind::InvalidLineContext
    );
}

#[test]
fn production_final_line_context_counts_real_objects_and_explicit_breaks() {
    let mut value = production_body_fixture(5_000_000);
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let span = children[0]["span"].clone();
    children.insert(
        1,
        serde_json::json!({"kind":"hard_break","node_id":0,"span":span}),
    );
    children.insert(
        2,
        serde_json::json!({"kind":"soft_break","node_id":0,"span":span}),
    );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, _, _| {
        let contexts = typaxis_layout::production_selected_line_contexts(lines).unwrap();
        assert_eq!(contexts.paragraphs()[0].ends(), [5, 9]);
        assert_eq!(contexts.paragraphs()[1].ends(), [1]);
    });
}

#[test]
fn production_final_line_driver_only_exposes_stable_selected_glyphs_and_shares_work_budget() {
    with_production_inline_context(
        &production_context_font_fixture("A B"),
        &config(),
        |prepared, package, profile, limits, admitted, bindings| {
            let flow = prepared.source_flow();
            let style = flow.paragraphs()[0].style().block_style();
            let indents = style.start_indent().get().raw() + style.end_indent().get().raw();
            let body = typaxis_core::Rect::new(
                Length::ZERO,
                Length::ZERO,
                PositiveLength::new(Length::from_raw(1_000_000 + indents).unwrap()).unwrap(),
                PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap(),
            );
            let work = typaxis_layout::with_converged_production_body_lines(
                package,
                flow.navigation(),
                profile,
                limits,
                admitted,
                flow,
                bindings,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                body,
                100_000,
                |stable| {
                    assert_eq!(stable.passes().len(), 2);
                    assert!(!stable.passes()[0].is_stable());
                    assert!(stable.passes()[1].is_stable());
                    assert_eq!(stable.lines().paragraphs()[0].lines().len(), 2);
                    assert!(stable.candidate_steps() > stable.lines().candidate_steps());
                    let glyphs = stable.lines().paragraphs()[0]
                        .lines()
                        .iter()
                        .flat_map(|l| l.items())
                        .flat_map(|item| match item {
                            typaxis_layout::ProductionPlacedInline::Text(t) => t.glyphs(),
                            _ => &[],
                        })
                        .map(|g| g.glyph().original_gid.get())
                        .collect::<Vec<_>>();
                    assert_eq!(glyphs, [2, 1, 3]);
                    stable.candidate_steps()
                },
            )
            .unwrap();
            for budget in [work - 1, work] {
                let called = std::cell::Cell::new(false);
                let result = typaxis_layout::with_converged_production_body_lines(
                    package,
                    flow.navigation(),
                    profile,
                    limits,
                    admitted,
                    flow,
                    bindings,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    body,
                    budget,
                    |_| called.set(true),
                );
                assert_eq!(result.is_ok(), budget == work);
                assert_eq!(called.get(), budget == work);
            }
        },
    );
}

#[test]
fn production_final_line_driver_never_calls_consumer_for_actual_oscillation() {
    let cfg = config_with_limits(ResourceLimits {
        max_line_reshape_passes: 2,
        ..ResourceLimits::default()
    });
    with_production_inline_context(
        &production_context_font_fixture("A B"),
        &cfg,
        |prepared, package, profile, limits, admitted, bindings| {
            let flow = prepared.source_flow();
            let style = flow.paragraphs()[0].style().block_style();
            let indents = style.start_indent().get().raw() + style.end_indent().get().raw();
            let body = typaxis_core::Rect::new(
                Length::ZERO,
                Length::ZERO,
                PositiveLength::new(Length::from_raw(1_300_000 + indents).unwrap()).unwrap(),
                PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap(),
            );
            let failure = typaxis_layout::with_converged_production_body_lines(
                package,
                flow.navigation(),
                profile,
                limits,
                admitted,
                flow,
                bindings,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                body,
                100_000,
                |_| panic!("unstable layout must not reach the consumer"),
            )
            .unwrap_err();
            assert!(matches!(
                failure,
                typaxis_layout::ProductionBodyReshapeError::Feedback(
                    typaxis_linebreak::BreakError::IterationLimit
                )
            ));
        },
    );
}
