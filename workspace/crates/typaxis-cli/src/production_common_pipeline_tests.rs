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
        assert_eq!(error.kind, FailureKind::Input);
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
