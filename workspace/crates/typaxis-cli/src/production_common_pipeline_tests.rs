#[test]
fn production_common_driver_closes_stable_lines_and_actual_block_terminals() {
    for value in [
        production_body_fixture(5_000_000),
        production_numbered_body_fixture(5_000_000),
    ] {
        with_production_inline_tagged_context(
            &serde_json::to_vec(&value).unwrap(),
            &config(),
            |prepared, package, _, limits, admitted, _, semantics, profile| {
                assert!(package.math_nodes().is_empty());
                let (hash, observation) = with_production_common_body_pdf(
                    package,
                    prepared.source_flow().navigation(),
                    semantics,
                    profile,
                    admitted,
                    limits,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    100_000,
                    |pdf, page_stability, observation| {
                        assert_eq!(page_stability.passes().len(), observation.page_passes);
                        assert!(pdf.bytes().starts_with(b"%PDF-1.7"));
                        assert!(pdf.page_count() > 0);
                        assert!(!pdf.objects().is_empty());
                        assert!(observation.line_reshape_passes >= 2);
                        assert_eq!(observation.page_passes, 2);
                        assert!(observation.page_record_charge > 0);
                        assert_eq!(observation.block_math_terminals, 1);
                        assert!(observation.candidate_steps > 0);
                        Ok((pdf.content_hash(), observation))
                    },
                )
                .unwrap();
                let repeated = with_production_common_body_pdf(
                    package,
                    prepared.source_flow().navigation(),
                    semantics,
                    profile,
                    admitted,
                    limits,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    100_000,
                    |pdf, page_stability, repeated| {
                        assert_eq!(page_stability.passes().len(), repeated.page_passes);
                        assert_eq!(repeated, observation);
                        Ok(pdf.content_hash())
                    },
                )
                .unwrap();
                assert_eq!(hash, repeated);
            },
        );
    }
}

#[test]
fn production_common_driver_never_exposes_incomplete_selection() {
    let value = production_body_fixture(5_000_000);
    with_production_inline_tagged_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, package, _, limits, admitted, _, semantics, profile| {
            let mut inspected = false;
            let result = with_production_common_body_pdf(
                package,
                prepared.source_flow().navigation(),
                semantics,
                profile,
                admitted,
                limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                0,
                |_, _, _| {
                    inspected = true;
                    Ok(())
                },
            );
            assert!(result.is_err());
            assert!(!inspected);
        },
    );
}

#[test]
#[ignore = "requires explicit saved VMB job and diagnostic PDF output"]
fn production_common_driver_saved_vmb_job() {
    let job = PathBuf::from(std::env::var_os("TYPAXIS_COMMON_BODY_JOB").expect("explicit job"));
    let output =
        PathBuf::from(std::env::var_os("TYPAXIS_COMMON_BODY_PDF").expect("explicit output"));
    assert!(job.is_absolute() && output.is_absolute());
    let bytes = fs::read(job.join("document-package.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let (package, navigation, limits, admitted) =
        production_text_fixture_at(&bytes, &config(), &job);
    for source in value["sources"].as_array().unwrap() {
        let uri = source["uri"].as_str().unwrap();
        assert!(!Path::new(uri).is_absolute());
        assert!(Path::new(uri)
            .components()
            .all(|c| matches!(c, Component::Normal(_))));
        let data = fs::read(job.join(uri)).unwrap();
        assert_eq!(
            data.len() as u64,
            source["utf8_byte_length"].as_u64().unwrap()
        );
        let digest: String = sha256(&data).iter().map(|v| format!("{v:02x}")).collect();
        assert_eq!(digest, source["sha256"].as_str().unwrap());
    }
    assert!(package.math_nodes().is_empty());
    let semantics =
        typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
            .unwrap();
    let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
    let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
        &package,
        &navigation,
        &semantics,
        &limits,
        &identity,
    )
    .unwrap();
    with_production_common_body_pdf(
        &package,
        &navigation,
        &semantics,
        &profile,
        &admitted,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        1_000_000,
        |pdf, page_stability, observation| {
            assert_eq!(page_stability.passes().len(), observation.page_passes);
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)
                .unwrap();
            file.write_all(pdf.bytes()).unwrap();
            file.sync_all().unwrap();
            println!(
                "common_body_pdf_sha256={:02x?} pages={} observation={observation:?}",
                pdf.content_hash(),
                pdf.page_count()
            );
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn production_common_footnote_driver_closes_actual_source_to_pdf() {
    for value in [
        production_body_navigation_vmb_fixture(),
        production_footnote_flow_fixture(),
        production_footnote_numbered_definition_fixture(),
        production_footnote_two_long_definitions(),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &config());
        let semantics =
            typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
                .unwrap();
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &identity,
        )
        .unwrap();
        let mut previous = None;
        let mut used_steps = 0;
        for _ in 0..2 {
            let result = with_production_common_footnote_pdf(
                &package,
                &navigation,
                &semantics,
                &profile,
                &admitted,
                &limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                100_000,
                |pdf, stable, book_inputs, book_pdf, observation| {
                    assert!(pdf.bytes().starts_with(b"%PDF-1.7"));
                    assert_eq!(pdf.page_count() as usize, stable.sequence().pages().len());
                    assert!(observation.line_reshape_passes >= 2);
                    assert_eq!(observation.page_passes, stable.passes());
                    assert!(observation.page_passes >= 2);
                    assert_eq!(observation.record_charge, book_pdf.record_charge());
                    assert_eq!(observation.spool_charge, book_pdf.spool_charge());
                    assert!(observation.record_charge > pdf.record_charge());
                    assert_eq!(
                        book_inputs.selected().pages().len(),
                        pdf.page_count() as usize
                    );
                    assert!(observation.page_work_steps >= stable.work_steps());
                    assert!(observation.line_candidate_steps > 0);
                    used_steps = observation.line_candidate_steps + observation.page_work_steps;
                    assert!(used_steps <= 100_000);
                    Ok((pdf.bytes().to_vec(), observation))
                },
            )
            .unwrap();
            if let Some(previous) = &previous {
                assert_eq!(&result, previous);
            }
            previous = Some(result);
        }
        for budget in [0, used_steps - 1, used_steps] {
            let mut called = false;
            let result = with_production_common_footnote_pdf(
                &package,
                &navigation,
                &semantics,
                &profile,
                &admitted,
                &limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                budget,
                |pdf, _, _, _, observation| {
                    called = true;
                    assert_eq!(pdf.bytes(), previous.as_ref().unwrap().0);
                    assert_eq!(observation, previous.as_ref().unwrap().1);
                    Ok(())
                },
            );
            if budget == 0 || budget == used_steps - 1 {
                let error = result.as_ref().unwrap_err();
                assert_eq!(error.kind, FailureKind::Limit, "{error:?}");
                assert!(error.message.starts_with("L5110:"), "{error:?}");
            }
            assert_eq!(
                result.is_ok(),
                budget == used_steps,
                "budget={budget}, error={:?}",
                result.err()
            );
            assert_eq!(called, budget == used_steps);
        }
    }
}

#[test]
fn production_common_footnote_driver_never_exposes_partial_pages() {
    for no_fit in [false, true] {
        let mut value = production_footnote_joint_geometry_fixture();
        let cfg = if no_fit {
            value["page_masters"]["masters"][0]["footnote"]["height"] = 1.into();
            config()
        } else {
            config_with_limits(ResourceLimits {
                max_pages: 1,
                ..ResourceLimits::default()
            })
        };
        let bytes = serde_json::to_vec(&value).unwrap();
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &cfg);
        let semantics =
            typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
                .unwrap();
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &identity,
        )
        .unwrap();
        let mut called = false;
        let error = with_production_common_footnote_pdf(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            100_000,
            |_, _, _, _, _| {
                called = true;
                Ok(())
            },
        )
        .unwrap_err();
        assert!(!called);
        assert_eq!(
            error.kind,
            if no_fit {
                FailureKind::Input
            } else {
                FailureKind::Limit
            }
        );
        assert!(error
            .message
            .starts_with(if no_fit { "L5100:" } else { "L5110:" }));
        assert!(error.message.contains("node "));
        assert!(
            error.message.contains(if no_fit {
                "JointPageNoFit"
            } else {
                "PageLimit"
            }),
            "{}",
            error.message
        );
    }
}

#[test]
fn production_common_footnote_pdf_limits_keep_limit_exit_and_hide_partial_output() {
    let value = production_body_navigation_vmb_fixture();
    let bytes = serde_json::to_vec(&value).unwrap();
    let mut object_count = 0;
    let mut output_bytes = 0;
    let mut spool_bytes = 0;
    let mut record_count = 0;
    for mode in 0..5 {
        let mut resource_limits = ResourceLimits::default();
        match mode {
            1 => resource_limits.max_pdf_objects = object_count - 1,
            2 => resource_limits.max_output_bytes = output_bytes - 1,
            3 => resource_limits.max_spool_bytes = spool_bytes - 1,
            4 => resource_limits.max_fragments = record_count - 1,
            _ => {}
        }
        let cfg = config_with_limits(resource_limits);
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &cfg);
        let semantics =
            typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
                .unwrap();
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &identity,
        )
        .unwrap();
        let mut called = false;
        let result = with_production_common_footnote_pdf(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            100_000,
            |pdf, _, _, book_pdf, _| {
                called = true;
                object_count = pdf.objects().len() as u32;
                output_bytes = pdf.bytes().len() as u64;
                spool_bytes = book_pdf.spool_charge();
                record_count = book_pdf.record_charge();
                Ok(())
            },
        );
        if mode == 0 {
            result.unwrap();
            assert!(called);
        } else {
            assert!(!called);
            let error = result.unwrap_err();
            assert_eq!(error.kind, FailureKind::Limit, "mode={mode}: {error:?}");
            assert_eq!(error.kind.exit_code(), 5);
            assert!(
                error.message.starts_with(match mode {
                    1 => "G6100:",
                    4 => "L5110:",
                    _ => "D8101:",
                }),
                "{error:?}"
            );
        }
    }
}

fn production_page_reference_fixture(blank_pages: u32) -> (serde_json::Value, NodeId) {
    use serde_json::json;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    value["document"]["blocks"][0]["anchor_id"] = "target".into();
    let paragraph = &mut value["document"]["blocks"][0]["blocks"][0];
    let span = paragraph["span"].clone();
    paragraph["children"].as_array_mut().unwrap().push(json!({
        "kind":"reference", "node_id":4, "span":span, "target":"target", "format":"page"
    }));
    for _ in 0..blank_pages {
        value["document"]["blocks"].as_array_mut().unwrap().insert(
            0,
            json!({"kind":"page_break", "node_id":0, "classes":[], "span":span}),
        );
    }
    production_body_renumber(&mut value["document"], &mut 0);
    let reference_owner = NodeId::new(
        value["document"]["blocks"][blank_pages as usize]["blocks"][0]["children"][1]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    (value, reference_owner)
}

#[test]
fn production_page_reference_candidates_shape_and_select_real_generated_digits() {
    for blank_pages in [0u32, 1, 11] {
        let (value, reference_owner) = production_page_reference_fixture(blank_pages);
        let actual_page = blank_pages + 1;
        let bytes = serde_json::to_vec(&value).unwrap();
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &config());
        let semantics =
            typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
                .unwrap();
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &identity,
        )
        .unwrap();
        let authorization = profile.base().base().authorization();
        let bindings = typaxis_layout::bind_staging_precomposed_vectors(
            &package,
            authorization,
            &limits,
            &admitted,
        )
        .unwrap();
        let mut fingerprints = Vec::new();
        for page in [1, 12] {
            let flow = typaxis_syntax::prepare_production_text_flow_with_page_references(
                &package,
                &navigation,
                &limits,
                &[(reference_owner, page)],
            )
            .unwrap();
            let shaped = typaxis_shaping::shape_production_authored_text(
                &package,
                &navigation,
                &flow,
                &admitted,
                &limits,
                bindings.epoch().fingerprint(),
            )
            .unwrap();
            let prepared = typaxis_layout::prepare_production_inline_items(
                &package,
                &navigation,
                authorization,
                &limits,
                &admitted,
                &flow,
                &shaped,
                &bindings,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
            )
            .unwrap();
            assert_eq!(
                prepared.paragraphs()[0].glyph_clusters().len(),
                1 + page.to_string().len()
            );
            let width = PositiveLength::new(Length::from_raw(3_000_000).unwrap()).unwrap();
            let lines =
                typaxis_layout::layout_production_inline_lines(&prepared, &[width], 1000).unwrap();
            lines.verify(&prepared).unwrap();
            assert_eq!(
                flow.page_reference_text(reference_owner),
                Some(page.to_string().as_str())
            );
            fingerprints.push(flow.fingerprint());
            with_production_common_footnote_pdf_candidates(
                &package,
                &navigation,
                &semantics,
                &profile,
                &admitted,
                &limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                100_000,
                Some(&[(reference_owner, page)]),
                limits.base().get().max_layout_passes,
                |pdf, _, book, _, observed| {
                    assert!(observed.line_reshape_passes >= 2);
                    assert!(observed.page_passes >= 2);
                    assert_eq!(pdf.page_count(), actual_page);
                    // Resolve the actual root placement, including leading blank pages.
                    // Rendering a candidate must not make it page evidence.
                    let resolved = book
                        .resolved_page_references()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    assert_eq!(resolved, vec![(reference_owner, actual_page)]);
                    assert_eq!(
                        resolved == vec![(reference_owner, page)],
                        page == actual_page
                    );
                    let label = format!(
                        "/ActualText <FEFF{}>",
                        page.to_string()
                            .encode_utf16()
                            .map(|unit| format!("{unit:04X}"))
                            .collect::<String>()
                    );
                    assert!(pdf
                        .bytes()
                        .windows(label.len())
                        .any(|bytes| bytes == label.as_bytes()));
                    Ok(())
                },
            )
            .unwrap();
        }
        for seed in [1, 12] {
            let (expected_hash, convergence) = with_converged_production_page_reference_pdf(
                &package,
                &navigation,
                &semantics,
                &profile,
                &admitted,
                &limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                100_000,
                &[(reference_owner, seed)],
                |pdf, _, book, _, _, convergence| {
                    assert_eq!(pdf.page_count(), actual_page);
                    assert_eq!(
                        book.resolved_page_references()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap(),
                        vec![(reference_owner, actual_page)]
                    );
                    let label = format!(
                        "/ActualText <FEFF{}>",
                        actual_page
                            .to_string()
                            .encode_utf16()
                            .map(|unit| format!("{unit:04X}"))
                            .collect::<String>()
                    );
                    assert!(pdf
                        .bytes()
                        .windows(label.len())
                        .any(|bytes| bytes == label.as_bytes()));
                    assert_eq!(convergence.passes, if seed == actual_page { 2 } else { 3 });
                    assert_eq!(convergence.page_passes, convergence.passes * 2);
                    Ok((pdf.content_hash(), convergence))
                },
            )
            .unwrap();
            if seed == 1 {
                with_production_common_footnote_pdf(
                    &package,
                    &navigation,
                    &semantics,
                    &profile,
                    &admitted,
                    &limits,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    100_000,
                    |pdf, _, _, _, observed| {
                        assert_eq!(pdf.content_hash(), expected_hash);
                        assert_eq!(observed.page_passes, convergence.page_passes);
                        assert_eq!(
                            observed.line_reshape_passes,
                            convergence.line_reshape_passes
                        );
                        assert_eq!(
                            observed.line_candidate_steps,
                            convergence.line_candidate_steps
                        );
                        assert_eq!(observed.page_work_steps, convergence.page_work_steps);
                        assert_eq!(observed.record_charge, convergence.record_charge);
                        assert_eq!(observed.spool_charge, convergence.spool_charge);
                        Ok(())
                    },
                )
                .unwrap();
            }
            let work = convergence.line_candidate_steps + convergence.page_work_steps;
            for budget in [work - 1, work] {
                let mut inspected = false;
                let result = with_converged_production_page_reference_pdf(
                    &package,
                    &navigation,
                    &semantics,
                    &profile,
                    &admitted,
                    &limits,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    budget,
                    &[(reference_owner, seed)],
                    |pdf, _, _, _, _, repeated| {
                        inspected = true;
                        assert_eq!(pdf.content_hash(), expected_hash);
                        assert_eq!(repeated, convergence);
                        Ok(())
                    },
                );
                assert_eq!(inspected, budget == work);
                if budget == work {
                    result.unwrap();
                } else {
                    let error = result.unwrap_err();
                    assert_eq!(error.kind, FailureKind::Limit);
                    assert!(error.message.starts_with("L5110:"), "{error:?}");
                }
            }
        }
        assert_ne!(fingerprints[0], fingerprints[1]);
    }
}

#[test]
fn production_page_reference_convergence_enforces_cumulative_limits() {
    let (value, owner) = production_page_reference_fixture(0);
    let bytes = serde_json::to_vec(&value).unwrap();
    let mut records = 0;
    let mut spool = 0;
    for mode in 0..5 {
        let mut caps = ResourceLimits::default();
        match mode {
            1 => caps.max_layout_passes = 5,
            2 => caps.max_fragments = records - 1,
            3 => caps.max_spool_bytes = spool - 1,
            4 => {
                caps.max_layout_passes = 6;
                caps.max_fragments = records;
                caps.max_spool_bytes = spool;
            }
            _ => {}
        }
        let (package, navigation, limits, admitted) =
            production_text_fixture(&bytes, &config_with_limits(caps));
        let semantics =
            typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
                .unwrap();
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &identity,
        )
        .unwrap();
        let mut inspected = false;
        let result = with_converged_production_page_reference_pdf(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            100_000,
            &[(owner, 12)],
            |_, _, _, _, _, observation| {
                inspected = true;
                assert_eq!(observation.passes, 3);
                assert_eq!(observation.page_passes, 6);
                if mode == 0 {
                    records = observation.record_charge;
                    spool = observation.spool_charge;
                }
                Ok(())
            },
        );
        assert_eq!(inspected, mode == 0 || mode == 4);
        if inspected {
            result.unwrap();
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.kind, FailureKind::Limit, "mode {mode}: {error:?}");
            assert!(
                error
                    .message
                    .starts_with(if mode == 3 { "D8101:" } else { "L5110:" }),
                "{error:?}"
            );
        }
    }
}
