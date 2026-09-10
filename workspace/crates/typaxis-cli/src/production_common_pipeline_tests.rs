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
    let mut escaped_metadata = production_body_navigation_vmb_fixture();
    escaped_metadata["metadata"] = serde_json::json!({
        "author": "著者 & <共同> \"A\"",
        "created": "2026-09-08T01:02:03Z",
        "identifier": "urn:typaxis:metadata:source",
        "keywords": ["<数学> 😀", "日本語 & α"],
        "modified": "2026-09-08T04:05:06Z",
        "subject": "主題 > 補足",
        "title": "書籍 < & > \" 😀"
    });
    for value in [
        escaped_metadata,
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
                    let sealed = pdf
                        .seal_safe_vector(
                            &admitted,
                            &limits,
                            pdf.record_charge(),
                            pdf.spool_charge(),
                        )
                        .unwrap();
                    assert_eq!(sealed.closure().final_pdf_sha256(), pdf.content_hash());
                    assert_eq!(
                        sealed.closure().final_pdf_byte_length(),
                        pdf.bytes().len() as u64
                    );
                    assert_eq!(
                        sealed.closure().final_pdf_object_count(),
                        pdf.objects().len() as u32
                    );
                    assert_eq!(
                        sealed.closure().final_writer_observation_fingerprint(),
                        pdf.vector_final_writer().fingerprint()
                    );
                    let records = sealed.record_charge() - pdf.record_charge();
                    let spool = sealed.spool_charge() - pdf.spool_charge();
                    assert_eq!(spool, sealed.closure().canonical_jcs().len() as u64);
                    let record_base = limits.base().get().max_fragments - records;
                    let spool_base = limits.base().get().max_spool_bytes - spool;
                    let exact = pdf
                        .seal_safe_vector(&admitted, &limits, record_base, spool_base)
                        .unwrap();
                    assert_eq!(exact.record_charge(), limits.base().get().max_fragments);
                    assert_eq!(exact.spool_charge(), limits.base().get().max_spool_bytes);
                    assert_eq!(exact.closure(), sealed.closure());
                    assert_eq!(
                        pdf.seal_safe_vector(&admitted, &limits, record_base + 1, spool_base),
                        Err(typaxis_pdf::ProductionBodyAssemblyError::RecordLimit)
                    );
                    assert_eq!(
                        pdf.seal_safe_vector(&admitted, &limits, record_base, spool_base + 1),
                        Err(typaxis_pdf::ProductionBodyAssemblyError::SpoolLimit)
                    );
                    assert!(pdf.bytes().starts_with(b"%PDF-1.7"));
                    assert_eq!(pdf.page_count() as usize, stable.page_count());
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
    let mut first_closure: Option<typaxis_pdf::ProductionPageReferencePdfClosure> = None;
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
                    let closed = typaxis_pdf::seal_production_page_reference_pdf(
                        pdf,
                        book,
                        profile.base().authorization(),
                        &admitted,
                        &limits,
                        observed.record_charge,
                    );
                    if page == actual_page {
                        let closed = closed.unwrap();
                        assert_eq!(closed.reference_count(), 1);
                        assert_eq!(closed.pdf_sha256(), pdf.content_hash());
                        assert_eq!(closed.record_charge(), observed.record_charge + 1);
                        closed
                            .verify(
                                pdf,
                                book,
                                profile.base().authorization(),
                                &admitted,
                                &limits,
                            )
                            .unwrap();
                        if let Some(first) = &first_closure {
                            assert_eq!(
                                first.verify(
                                    pdf,
                                    book,
                                    profile.base().authorization(),
                                    &admitted,
                                    &limits
                                ),
                                Err(typaxis_pdf::ProductionBodyAssemblyError::ReceiptMismatch)
                            );
                        } else {
                            first_closure = Some(closed);
                        }
                        assert_eq!(
                            typaxis_pdf::seal_production_page_reference_pdf(
                                pdf,
                                book,
                                profile.base().authorization(),
                                &admitted,
                                &limits,
                                u64::MAX
                            ),
                            Err(typaxis_pdf::ProductionBodyAssemblyError::RecordLimit)
                        );
                    } else {
                        assert_eq!(
                            closed,
                            Err(typaxis_pdf::ProductionBodyAssemblyError::ReceiptMismatch)
                        );
                    }
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

#[test]
fn production_page_reference_reflow_moves_the_target_and_converges() {
    use serde_json::json;
    let (mut value, _) = production_page_reference_fixture(10);
    // The metrics-only fixture has empty ASCII outlines. Use the matching
    // diagnostic digit outlines so independent renderers can verify reference ink.
    value["resources"]["font_faces"][0]["uri"] = "body-list-visible.ttf".into();
    value["resources"]["font_faces"][0]["expected_sha256"] =
        "17857592837017395c9f22614b712f41d2ad6175a5b3a39c4d8aa879c99044c6".into();
    value["text_buffers"][0]["utf8"] = "A ".into();
    value["text_buffers"][0]["mappings"][0]["text_range"]["end_byte"] = 2.into();
    let root = &mut value["document"]["blocks"][10];
    root["anchor_id"] = serde_json::Value::Null;
    root["blocks"][0]["children"][0]["text_span"]["end_byte"] = 2.into();
    let mut target = root.clone();
    target["anchor_id"] = "target".into();
    target["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .pop();
    value["document"]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(target);
    let master = &mut value["page_masters"]["masters"][0];
    // A + space + one digit fits; two digits move to a second line.
    // One body line fits per page, so this moves the following anchor as well.
    master["body"]["width"] = json!(1_600_000);
    master["body"]["height"] = json!(1_100_000);
    master["footnote"] = serde_json::Value::Null;
    production_body_renumber(&mut value["document"], &mut 0);
    let owner = NodeId::new(
        value["document"]["blocks"][10]["blocks"][0]["children"][1]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
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
    for (candidate, expected_page) in [(1, 12), (12, 13), (13, 13)] {
        with_production_common_footnote_pdf_candidates(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            100_000,
            Some(&[(owner, candidate)]),
            limits.base().get().max_layout_passes,
            |pdf, _, book, _, _| {
                assert_eq!(pdf.page_count(), expected_page, "candidate {candidate}");
                assert_eq!(
                    book.resolved_page_references()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap(),
                    vec![(owner, expected_page)]
                );
                Ok(())
            },
        )
        .unwrap();
    }
    with_production_common_footnote_pdf(
        &package,
        &navigation,
        &semantics,
        &profile,
        &admitted,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        100_000,
        |pdf, _, book, _, observation| {
            assert_eq!(pdf.page_count(), 13);
            assert_eq!(
                book.resolved_page_references()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
                vec![(owner, 13)]
            );
            assert_eq!(observation.page_passes, 8);
            if let Some(path) = std::env::var_os("VMB_PAGE_REFERENCE_PDF_PROBE") {
                std::fs::write(path, pdf.bytes()).unwrap();
            }
            assert!(pdf
                .bytes()
                .windows(b"/ActualText <FEFF00310033>".len())
                .any(|bytes| bytes == b"/ActualText <FEFF00310033>"));
            Ok(())
        },
    )
    .unwrap();

    let cfg = config_with_limits(ResourceLimits {
        max_layout_passes: 7,
        ..ResourceLimits::default()
    });
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
    let mut inspected = false;
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
            inspected = true;
            Ok(())
        },
    )
    .unwrap_err();
    assert!(!inspected);
    assert_eq!(error.kind, FailureKind::Limit);
    assert!(error.message.starts_with("L5110:"), "{error:?}");
}

#[test]
fn production_page_reference_inside_footnote_converges_without_marker_aliasing() {
    use serde_json::json;
    let (mut value, _) = production_page_reference_fixture(11);
    let body = &mut value["document"]["blocks"][11]["blocks"][0];
    let reference = body["children"].as_array_mut().unwrap().pop().unwrap();
    let span = body["span"].clone();
    body["children"].as_array_mut().unwrap().push(json!({
        "kind":"footnote_reference", "node_id":0, "span":span, "footnote_id":"note"
    }));
    value["document"]["footnotes"] = json!([{
        "node_id":0, "span":span, "footnote_id":"note", "blocks":[{
            "kind":"paragraph", "node_id":0, "span":span, "classes":[], "children":[reference]
        }]
    }]);
    value["resources"]["font_faces"][0]["uri"] = "body-list-visible.ttf".into();
    value["resources"]["font_faces"][0]["expected_sha256"] =
        "17857592837017395c9f22614b712f41d2ad6175a5b3a39c4d8aa879c99044c6".into();
    production_body_renumber(&mut value["document"], &mut 0);
    let owner = NodeId::new(
        value["document"]["footnotes"][0]["blocks"][0]["children"][0]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
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
    with_production_common_footnote_pdf(
        &package,
        &navigation,
        &semantics,
        &profile,
        &admitted,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        100_000,
        |pdf, _, book, _, observed| {
            assert_eq!(pdf.page_count(), 12);
            assert_eq!(
                pdf.objects()
                    .iter()
                    .filter(|object| matches!(
                        object.role(),
                        typaxis_pdf::ProductionBodyAssemblyRole::Body(
                            typaxis_pdf::ProductionBodyObjectRole::LinkAnnotation(_)
                        )
                    ))
                    .count(),
                2
            );
            assert_eq!(observed.page_passes, 6);
            assert_eq!(
                book.resolved_page_references()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
                vec![(owner, 12)]
            );
            assert!(pdf
                .bytes()
                .windows(b"/ActualText <FEFF00310032>".len())
                .any(|bytes| bytes == b"/ActualText <FEFF00310032>"));
            if let Some(path) = std::env::var_os("VMB_FOOTNOTE_REFERENCE_PDF_PROBE") {
                std::fs::write(path, pdf.bytes()).unwrap();
            }
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn production_common_final_serializer_preserves_convergence_charges() {
    let value = production_page_reference_fixture(11).0;
    let bytes = serde_json::to_vec(&value).unwrap();
    let cfg = config();
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
            assert!(observed.page_passes > 2);
            let final_pdf = typaxis_pdf::write_production_common_tagged_pdf_after_assembly(
                pdf,
                &semantics,
                profile.authorization(),
                profile.base().authorization(),
                &admitted,
                &limits,
                cfg.fingerprint(),
                observed.record_charge,
                observed.spool_charge,
            )
            .unwrap();
            let direct = typaxis_pdf::write_production_common_tagged_pdf(
                pdf.source(),
                &semantics,
                profile.authorization(),
                profile.base().authorization(),
                &admitted,
                &limits,
                cfg.fingerprint(),
            )
            .unwrap();
            assert_eq!(final_pdf.final_pdf().bytes(), direct.final_pdf().bytes());
            assert_eq!(
                final_pdf.record_charge(),
                direct.record_charge() + observed.record_charge - pdf.source().record_charge()
            );
            assert_eq!(
                final_pdf.spool_charge(),
                direct.spool_charge() + observed.spool_charge - pdf.source().spool_charge()
            );
            assert_eq!(final_pdf.final_pdf().page_count(), pdf.page_count());
            for (records, spool) in [
                (pdf.record_charge() - 1, observed.spool_charge),
                (observed.record_charge, pdf.spool_charge() - 1),
            ] {
                assert_eq!(
                    typaxis_pdf::write_production_common_tagged_pdf_after_assembly(
                        pdf,
                        &semantics,
                        profile.authorization(),
                        profile.base().authorization(),
                        &admitted,
                        &limits,
                        cfg.fingerprint(),
                        records,
                        spool
                    )
                    .unwrap_err(),
                    typaxis_pdf::ProductionBodyAssemblyError::ReceiptMismatch
                );
            }
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn production_common_final_driver_and_manifests_share_exact_budgets() {
    for value in [
        production_body_navigation_vmb_fixture(),
        production_page_reference_fixture(11).0,
        production_table_fixture(),
        production_formula_header_table_fixture(),
        production_formula_header_table_with_tail_fixture(),
        production_numbered_formula_header_table_fixture(),
        production_inline_formula_header_table_fixture(),
        production_mixed_table_fixture(),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        let mut records = 0;
        let mut spool = 0;
        for mode in 0..5 {
            let cfg = config_with_limits(ResourceLimits {
                max_fragments: match mode {
                    1 => records,
                    2 => records - 1,
                    _ => ResourceLimits::default().max_fragments,
                },
                max_spool_bytes: match mode {
                    3 => spool,
                    4 => spool - 1,
                    _ => ResourceLimits::default().max_spool_bytes,
                },
                ..ResourceLimits::default()
            });
            let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &cfg);
            let semantics = typaxis_syntax::validate_staging_structure_semantics_v2(
                &package,
                &navigation,
                &limits,
            )
            .unwrap();
            let identity =
                typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
            let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
                &package,
                &navigation,
                &semantics,
                &limits,
                &identity,
            )
            .unwrap();
            let result = with_production_common_tagged_pdf(
                &package,
                &navigation,
                &semantics,
                &profile,
                &admitted,
                &limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                100_000,
                cfg.fingerprint(),
                |diagnostic, pdf, observed| {
                    assert_eq!(observed.record_charge, pdf.record_charge());
                    assert_eq!(observed.spool_charge, pdf.spool_charge());
                    assert_eq!(diagnostic.page_count(), pdf.final_pdf().page_count());
                    let marked = diagnostic
                        .source()
                        .structure_objects()
                        .annotations()
                        .marked();
                    let book = typaxis_manifest::build_production_book_navigation_manifest(
                        &package,
                        &navigation,
                        profile.base().authorization(),
                        &pdf,
                        &limits,
                    )
                    .map_err(|e| map_common_assembly_error("book manifest", e))?;
                    let safe = typaxis_manifest::build_production_safe_vector_manifest(
                        marked.content(),
                        profile.base().base().authorization(),
                        &book,
                        &pdf,
                        &admitted,
                        &limits,
                    )
                    .map_err(|e| map_common_assembly_error("safe manifest", e))?;
                    let math = typaxis_manifest::build_production_math_vector_manifest(
                        marked.structure().display(),
                        &safe,
                        &limits,
                    )
                    .map_err(|e| map_common_assembly_error("math manifest", e))?;
                    let tagged = typaxis_manifest::build_production_tagged_manifest(
                        marked.structure(),
                        &pdf,
                        &safe,
                        &math,
                        &limits,
                    )
                    .map_err(|e| map_common_assembly_error("tagged manifest", e))?;
                    Ok((tagged.record_charge(), tagged.spool_charge()))
                },
            );
            if mode == 2 || mode == 4 {
                let error = result.unwrap_err();
                assert_eq!(error.kind, FailureKind::Limit, "{}", error.message);
            } else {
                let charge = result.unwrap();
                if mode == 0 {
                    (records, spool) = charge;
                } else {
                    assert_eq!(charge, (records, spool));
                }
            }
        }
    }
}

#[test]
fn production_public_book_uses_common_selection_and_manifest_chain() {
    for (name, value) in [
        ("navigation", production_body_navigation_vmb_fixture()),
        ("reference", production_page_reference_fixture(11).0),
        ("single-page-table", production_table_fixture()),
        ("table-header", production_formula_header_table_fixture()),
        ("table-header-tail", production_formula_header_table_with_tail_fixture()),
        ("table-header-numbered", production_numbered_formula_header_table_fixture()),
        ("table-header-inline", production_inline_formula_header_table_fixture()),
        ("table-footnotes", production_mixed_table_fixture()),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        let cfg = EffectiveConfig::new_for_contract(DocumentPackageContractId::V1_4,
            false, PdfStreamCompression::None, vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto", "tel"].map(str::to_owned).to_vec(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default()).unwrap();
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
        let built = build_production_book_pdf(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            &cfg,
        )
        .unwrap();
        let (pdf, fields, selected, _, fragments, trace, pass_count) = built.into_parts();
        assert_eq!(pass_count.get(), if name == "reference" { 6 } else { 2 });
        if let Some(table) = value["document"]["blocks"][0]["blocks"].as_array().and_then(|blocks| blocks.iter().find(|block| block["kind"] == "table")) {
            let text = String::from_utf8_lossy(pdf.bytes());
            assert_eq!(text.matches("/S /Table ").count(), 1);
            let cells = |section: &str| table[section].as_array().unwrap().iter().map(|row| row["cells"].as_array().unwrap().len()).sum::<usize>();
            assert_eq!(text.matches("/S /TH ").count(), cells("head"));
            assert_eq!(text.matches("/S /TD ").count(), cells("body"));
            if let Some(dir) = std::env::var_os("VMB_SINGLE_PAGE_TABLE_OUT") {
                let dir = PathBuf::from(dir);
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                fs::write(dir.join(format!("{name}-package.json")), &bytes).unwrap();
            }
        }
        assert_eq!(pdf.selected_layout_fingerprint().bytes(), selected);
        assert!(fragments > 0);
        assert!(pdf.footnote_display_sha256().is_some());
        assert!(fields.book_navigation_record().is_some());
        assert!(fields.safe_vector_record().is_some());
        assert!(fields.math_vector_record().is_some());
        assert!(fields.tagged_pdf_record().is_some());
        let trace: serde_json::Value = serde_json::from_str(&trace).unwrap();
        assert_eq!(trace["fragment_count"], fragments);
        assert_eq!(trace["pass_count"], pass_count.get());
        assert_eq!(trace["selected_state"], pass_count.get());
        assert!(trace["native_math_layout_sha256"].is_null());
        let has_table = value["document"]["blocks"][0]["blocks"].as_array()
            .is_some_and(|blocks| blocks.iter().any(|block| block["kind"] == "table"));
        assert_eq!(trace["table_measurements_sha256"].is_string(), has_table);
        if !has_table {
            assert!(trace["table_measurements_sha256"].is_null());
        }
        let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(trace["selected_layout_sha256"], hex(selected));
        assert_eq!(
            trace["vector_display_sha256"],
            hex(pdf.footnote_display_sha256().unwrap())
        );
        let tagged: serde_json::Value =
            serde_json::from_str(fields.tagged_pdf_record().unwrap()).unwrap();
        assert_eq!(
            tagged["fingerprints"]["pdf_sha256"],
            hex(pdf.content_hash())
        );
        assert!(pdf
            .bytes()
            .windows(b"<pdfuaid:part>1</pdfuaid:part>".len())
            .any(|w| w == b"<pdfuaid:part>1</pdfuaid:part>"));
    }
}

#[test]
fn production_native_math_public_pdf_keeps_formula_and_embedded_font() {
    for (name, value) in [("native", production_native_math_fixture()), ("fraction", production_native_math_fraction_fixture())] {
    let expected_speech = value["document"]["blocks"][0]["children"][0]["speech"].as_str().unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    let cfg = EffectiveConfig::new_for_contract(DocumentPackageContractId::V1_4,
        false, PdfStreamCompression::None, vec![ConfigResourceRoot::ProjectRoot],
        ["http", "https", "mailto", "tel"].map(str::to_owned).to_vec(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
        ResourceLimits::default()).unwrap();
    let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &cfg);
    let semantics = typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits).unwrap();
    let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
    let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
        &package, &navigation, &semantics, &limits, &identity,
    ).unwrap();
    let built = build_production_book_pdf(&package, &navigation, &semantics, &profile, &admitted, &limits, &cfg).unwrap();
    let (pdf, fields, selected, _, fragments, trace, _) = built.into_parts();
    let native = typaxis_layout::prepare_production_native_math_context(
        &package, profile.base().base().authorization(), &limits, &admitted,
    ).unwrap().unwrap();
    let trace: serde_json::Value = serde_json::from_str(&trace).unwrap();
    assert_eq!(trace["native_math_layout_sha256"], native.computations().fingerprint()
        .iter().map(|b| format!("{b:02x}")).collect::<String>());
    assert!(trace["table_measurements_sha256"].is_null());
    assert_eq!(pdf.selected_layout_fingerprint().bytes(), selected);
    assert!(fragments > 0);
    assert!(fields.tagged_pdf_record().is_some());
    let content = String::from_utf8_lossy(pdf.bytes());
    assert!(content.contains("/S /Formula"));
    assert!(content.contains("/FontFile2"));
    assert!(content.contains("/PB0"));
    let speech: String = expected_speech.encode_utf16().map(|c| format!("{c:04X}")).collect();
    assert!(content.contains(&format!("/ActualText <FEFF{speech}>")));
    if let Ok(directory) = std::env::var("VMB_NATIVE_PDF_OUT") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
        fs::write(directory.join(format!("{name}-package.json")), bytes).unwrap();
    }
    }
}
#[test]
fn production_native_math_context_reuses_receipts_across_convergence() {
    let value = production_native_math_fixture();
    with_production_inline_tagged_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, package, authorization, limits, admitted, bindings, semantics, tagged| {
            let native = typaxis_layout::prepare_production_native_math_context(
                package,
                authorization,
                limits,
                admitted,
            )
            .unwrap()
            .unwrap();
            let copied_ledger = admitted.clone();
            assert!(native.verify_source(package, authorization, limits, &copied_ledger).is_err());
            let work = native.computations().layout_work();
            assert!(work > 0);
            let flow = prepared.source_flow();
            let mut previous = None;
            for _ in 0..2 {
                typaxis_layout::with_converged_production_body_lines_with_native_context(
                    package,
                    flow.navigation(),
                    authorization,
                    limits,
                    admitted,
                    flow,
                    bindings,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    authorization.page_geometry().body(),
                    100_000,
                    Some(&native),
                    |stable| {
                        assert!(!stable.passes().is_empty());
                        let mut count = 0;
                        for paragraph in stable.lines().paragraphs() {
                            for line in paragraph.lines() {
                                for item in line.items() {
                                    if let typaxis_layout::ProductionPlacedInline::Math(math) = item
                                    {
                                        assert!(std::ptr::eq(
                                            math.receipt(),
                                            native.computations().receipt(math.owner()).unwrap()
                                        ));
                                        count += 1;
                                    }
                                }
                            }
                        }
                        assert_eq!(count, 1);
                        let hash = stable.lines().fingerprint();
                        if let Some(previous) = previous {
                            assert_eq!(hash, previous);
                        }
                        previous = Some(hash);
                    },
                )
                .unwrap();
            }
            for _ in 0..2 {
                with_production_common_footnote_pdf_candidates_with_native_context(
                    package,
                    flow.navigation(),
                    semantics,
                    tagged,
                    admitted,
                    limits,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    100_000,
                    None,
                    limits.base().get().max_layout_passes,
                    Some(&native),
                    |pdf, _, _, _, _| {
                        assert!(pdf.bytes().starts_with(b"%PDF-1.7"));
                        Ok(())
                    },
                )
                .unwrap();
            }
            assert_eq!(native.computations().layout_work(), work);
            // A missing context must fail closed before the stable callback.
            assert!(
                typaxis_layout::with_converged_production_body_lines_with_native_context(
                    package,
                    flow.navigation(),
                    authorization,
                    limits,
                    admitted,
                    flow,
                    bindings,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    authorization.page_geometry().body(),
                    100_000,
                    None,
                    |_| panic!("missing computation must not select lines"),
                )
                .is_err()
            );
            // Same document under a separately issued profile is a foreign session.
            let identity =
                typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
            let foreign = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
                package,
                flow.navigation(),
                semantics,
                limits,
                &identity,
            )
            .unwrap();
            assert!(native
                .verify_source(
                    package,
                    foreign.base().base().authorization(),
                    limits,
                    admitted
                )
                .is_err());
        },
    );
}
#[test]
fn production_native_math_and_page_reference_converge_at_exact_math_work_limit() {
    let mut value = production_native_math_fixture();
    let paragraph = &mut value["document"]["blocks"][0];
    let span = paragraph["span"].clone();
    let children = paragraph["children"].as_array_mut().unwrap();
    children.insert(
        0,
        serde_json::json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":"native-target"}),
    );
    children.push(serde_json::json!({"kind":"reference","node_id":0,"span":span,"target":"native-target","format":"page"}));
    production_body_renumber(&mut value["document"], &mut 0);
    let owner = NodeId::new(
        value["document"]["blocks"][0]["children"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    let bytes = serde_json::to_vec(&value).unwrap();
    let (package, _, _, _) = production_text_fixture(&bytes, &config());
    let work: u64 = package
        .math_nodes()
        .iter()
        .map(|node| typaxis_math::required_math_layout_units(node.parsed()).unwrap())
        .sum();
    assert!(work > 1);
    for maximum in [work, work - 1] {
        let mut overrides = crate::config::ConfigOverrides::default();
        overrides
            .set_limit("max_math_layout_units", maximum)
            .unwrap();
        overrides.no_compress = true;
        let cfg = crate::config::load_for_profile(
            typaxis_core::MachinePdfProfileId::ProductionBook1,
            None,
            Vec::<(&str, &str)>::new(),
            &overrides,
        )
        .unwrap();
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
            &[(owner, 99)],
            |pdf, _, _, _, _, total| {
                inspected = true;
                assert!(
                    total.passes >= 3,
                    "candidate 99 must change to the actual page and stabilize"
                );
                assert!(total.line_reshape_passes >= usize::from(total.passes));
                let pdf = String::from_utf8_lossy(pdf.bytes());
                assert!(pdf.contains("/S /Formula"));
                assert!(pdf.contains("/FontFile2"));
                Ok(())
            },
        );
        if maximum == work {
            result.unwrap();
            assert!(inspected);
            let built = build_production_book_pdf(
                &package, &navigation, &semantics, &profile, &admitted, &limits, &cfg,
            ).unwrap();
            let (pdf, _, _, _, _, _, _) = built.into_parts();
            if let Ok(directory) = std::env::var("VMB_NATIVE_PDF_OUT") {
                let directory = PathBuf::from(directory);
                fs::create_dir_all(&directory).unwrap();
                fs::write(directory.join("native-page-reference.pdf"), pdf.bytes()).unwrap();
                fs::write(directory.join("native-page-reference-package.json"), &bytes).unwrap();
                fs::write(directory.join("native-page-reference-work.txt"), work.to_string()).unwrap();
            }

        } else {
            let error = result.unwrap_err();
            assert_eq!(error.kind, FailureKind::Limit);
            assert!(error.message.starts_with("L5111:"), "{error:?}");
            assert!(!inspected);
        }
    }
}

#[test]
fn production_common_book_numbered_label_clusters_share_one_language_occurrence() {
    let value = production_numbered_body_text_fixture(5_000_000, "AB");
    with_production_inline_tagged_context(&serde_json::to_vec(&value).unwrap(), &config(),
        |prepared, package, _, limits, admitted, _, semantics, profile| {
            with_production_common_footnote_pdf(package, prepared.source_flow().navigation(), semantics,
                profile, admitted, limits, typaxis_linebreak::JapaneseLineBreakMode::Normal, 100_000,
                |pdf, _, book, _, _| {
                    let structure = pdf.source().structure_objects().annotations().marked().structure();
                    let draws = structure.display().draws().iter().filter_map(|draw| {
                        let typaxis_display_list::ProductionBodyDraw::Text(text) = draw else { return None; };
                        text.equation_number().map(|_| text)
                    }).collect::<Vec<_>>();
                    assert_eq!(draws.iter().map(|text| text.exact_text()).collect::<String>(), "AB");
                    assert!(draws.len() > 1);
                    assert_eq!(book.child_language_paints().len(), 1);
                    assert_eq!(book.child_language_paints()[0].owner_node_id(), draws[0].owner());
                    Ok(())
                }).unwrap();
        });
}

#[test]
fn production_common_combined_preserves_tall_table_rows_and_authored_page_break() {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
    let (package, navigation, limits, admitted) = production_text_fixture(bytes, &config());
    let semantics = typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits).unwrap();
    let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
    let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(&package, &navigation, &semantics, &limits, &identity).unwrap();
    with_production_common_footnote_pdf(&package, &navigation, &semantics,
                &profile, &admitted, &limits, typaxis_linebreak::JapaneseLineBreakMode::Normal, 1_000_000,
                |pdf, _, _, _, _| {
                    let display = pdf.source().structure_objects().annotations().marked().structure().display();
                    let source = display.source();
                    let measured = typaxis_pagination::prepare_production_table_measurements(source.line_layout(), source.block_layout(), source.footnote_lines(), &limits).unwrap();
                    assert_eq!(measured.tables().len(), 1);
                    let table = &measured.tables()[0];
                    assert_eq!(table.owner().get(), 23);
                    assert_eq!(table.rows().iter().map(|row| (row.owner().get(), row.height().raw())).collect::<Vec<_>>(), [(24, 1_048_576), (31, 8_131_072), (38, 8_131_072), (42, 8_131_072)]);
                    assert_eq!(table.height().raw(), 25_441_792);
                    let body = source.block_layout().page_geometry().body();
                    assert!(table.height() > body.height().get());
                    let geometry = source.geometry().mixed().unwrap();
                    assert_eq!(geometry.sequence().measurements_fingerprint(), measured.fingerprint());
                    assert_eq!(geometry.sequence().pages()[0].forced_break().unwrap().get(), 19);
                    assert_eq!(pdf.page_count(), 5);
                    let mut tall = std::collections::BTreeMap::new();
                    for page in geometry.pages() {
                        for placed in page.fragments() {
                            let fragment = placed.fragment();
                            if [33, 36, 40, 44].contains(&fragment.owner().get()) {
                                assert_eq!(fragment.bounds().height().get().raw(), 8_000_000);
                                assert!(fragment.bounds().y() >= body.y());
                                assert!(fragment.bounds().y().checked_add(fragment.bounds().height().get()).unwrap() <= body.y().checked_add(body.height().get()).unwrap());
                                assert!(tall.insert(fragment.owner().get(), fragment.page_index()).is_none());
                            }
                        }
                    }
                    assert_eq!(tall, [(33, 1), (36, 1), (40, 2), (44, 2)].into_iter().collect());
                    Ok(())
                }).unwrap();
}


#[test]
fn production_public_pass_summary_counts_all_page_reference_retries() {
    for (blank_pages, expected_passes) in [(0, 4), (1, 6), (11, 6)] {
        let (value, _) = production_page_reference_fixture(blank_pages);
        let cfg = EffectiveConfig::new_for_contract(
            DocumentPackageContractId::V1_4, false, PdfStreamCompression::None,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto", "tel"].map(str::to_owned).to_vec(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default(),
        ).unwrap();
        let (package, navigation, limits, admitted) =
            production_text_fixture(&serde_json::to_vec(&value).unwrap(), &cfg);
        let semantics = typaxis_syntax::validate_staging_structure_semantics_v2(
            &package, &navigation, &limits,
        ).unwrap();
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package, &navigation, &semantics, &limits, &identity,
        ).unwrap();
        let built = build_production_book_pdf(
            &package, &navigation, &semantics, &profile, &admitted, &limits, &cfg,
        ).unwrap();
        let (pdf, _, selected, _, _, trace, count) = built.into_parts();
        assert_eq!(pdf.page_count(), blank_pages + 1);
        assert_eq!(pdf.selected_layout_fingerprint().bytes(), selected);
        assert_eq!(count.get(), expected_passes);
        let trace: serde_json::Value = serde_json::from_str(&trace).unwrap();
        assert_eq!(trace["pass_count"], expected_passes);
        assert_eq!(trace["selected_state"], expected_passes);
    }
}

#[test]
fn production_common_failure_keeps_typed_shaping_owner_and_canonical_reason() {
    use typaxis_shaping::{ProductionTextShapeError, ProductionTextShapeErrorKind as S};
    use typaxis_diagnostics::DiagnosticLocation;
    let owner = NodeId::new(37);
    for (kind, expected_code, reason, exit) in [
        (S::MissingDeclaredFontCoverage, "L5100", "missing_declared_font_coverage", 1),
        (S::MissingGeneratedGlyph, "L5100", "missing_generated_glyph", 1),
        (S::OutputLimit, "L5110", "output_limit", 5),
        (S::ReceiptMismatch, "I9190", "receipt_mismatch", 4),
    ] {
        let failure = map_common_reshape_error(typaxis_layout::ProductionBodyReshapeError::Shape(
            ProductionTextShapeError { owner, kind },
        ));
        assert_eq!(failure.kind.exit_code(), exit);
        let diagnostic = failure.processing_diagnostic().unwrap();
        assert_eq!(diagnostic.code().as_str(), expected_code);
        assert_eq!(diagnostic.message(), "production text shaping failed");
        let Some(DiagnosticLocation::Source(location)) = diagnostic.location() else { panic!("typed source location lost") };
        assert_eq!(location.node_id(), Some(owner));
        assert_eq!(location.text_span(), None);
        assert_eq!(location.source_span(), None);
        assert_eq!(diagnostic.notes()[0].message(), format!("phase=authored-text-shaping; reason={reason}"));
    }
}

#[test]
fn production_common_failure_preserves_missing_glyph_text_span() {
    use typaxis_core::{TextBufferId, TextSpan, Utf8ByteOffset};
    use typaxis_shaping::{ProductionTextShapeError, ProductionTextShapeErrorKind};
    let span = TextSpan::new(TextBufferId::new(4), Utf8ByteOffset::new(3), Utf8ByteOffset::new(7)).unwrap();
    let failure = map_common_reshape_error(typaxis_layout::ProductionBodyReshapeError::Shape(
        ProductionTextShapeError { owner: NodeId::new(9), kind: ProductionTextShapeErrorKind::MissingShapedGlyph { span } },
    ));
    let diagnostic = failure.processing_diagnostic().unwrap();
    let Some(typaxis_diagnostics::DiagnosticLocation::Source(location)) = diagnostic.location() else { panic!("missing source location") };
    assert_eq!(location.node_id(), Some(NodeId::new(9)));
    assert_eq!(location.text_span(), Some(span));
    assert_eq!(location.source_span(), None);
    assert!(diagnostic.notes()[0].message().ends_with("reason=missing_shaped_glyph"));
}
