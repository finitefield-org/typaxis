use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_pdf as pdf_drive;
use typaxis_pdf::book_v2::{
    BookV2ColumnPdfAssemblyBuilder as ColumnAssembly, BookV2FontStreamError as PdfResourceError,
    BookV2PdfAssemblyError as PdfError,
};
#[path = "book_v2_column_pdf_probes.rs"]
mod pdf_probes;

#[path = "book_v2_column_pdf_manual_tests.rs"]
mod manual_tests;
#[path = "book_v2_column_pdf_media_tests.rs"]
mod media_tests;
#[path = "book_v2_column_page_label_capture_tests.rs"]
mod page_label_capture_tests;

fn pdf_quartet(value: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [
        value.record_charge(),
        value.spool_charge(),
        value.output_charge(),
        value.work_steps(),
    ]
}

#[test]
fn book_v2_column_pdf_frame_view_rejects_another_named_master_with_identical_body_geometry() {
    use typaxis_display_list::book_v2::{BookV2AssemblyDisplay, BookV2AssemblyFrameError};
    let caps = display_caps();
    let mut data = region_data(2, false, false);
    let mut alternate = data["page_masters"]["masters"][0].clone();
    alternate["master_id"] = "same-body-other-running-owner".into();
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(alternate);
    data["page_masters"]["selection_rules"] = json!([
        {"master_id":"same-body-other-running-owner","parity":"any","first":null,"named_page":"other-running-owner","source_order":0}
    ]);
    let mut next = 0;
    semantic_renumber(&mut data["document"], &mut next);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            semantic_renumber(&mut master[role], &mut next);
        }
    }
    let root = Root::new();
    let input = display_input(&root, data, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    pdf_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
        let display = bundle.display();
        let view = display.assembly_source();
        let original = display
            .source()
            .original_graph()
            .lines()
            .prepared()
            .source_flow()
            .body();
        let selected = view.select_page_master(0, &mut 0, MAXIMUM).unwrap();
        let wrong = typaxis_syntax::book_v2::select_book_v2_page_master(
            original,
            0,
            Some("other-running-owner"),
            &mut 0,
            MAXIMUM,
        )
        .unwrap();
        assert_eq!(selected.master().body, wrong.master().body);
        assert_eq!(
            selected.advanced().column_layout,
            wrong.advanced().column_layout
        );
        assert_ne!(selected.master().master_id, wrong.master().master_id);
        assert!(!std::ptr::eq(selected.master(), wrong.master()));
        let mut work = 0;
        view.validate_page_frame(0, selected, &mut work, MAXIMUM)
            .unwrap();
        assert!(work > 0);
        assert!(matches!(
            view.validate_page_frame(0, wrong, &mut 0, MAXIMUM),
            Err(BookV2AssemblyFrameError::Mismatch)
        ));
    })
    .unwrap();
}

fn original_corpus_limits(canonical: &str) -> M4EffectiveResourceLimits {
    let wire: Value = serde_json::from_str(canonical).unwrap();
    let mut base = ResourceLimits::default();
    let mut extension = M4ResourceLimits::default();
    macro_rules! fields { ($target:ident,$section:literal,$($field:ident),+ $(,)?) => {$(
        $target.$field = wire[$section][stringify!($field)].as_u64().unwrap().try_into().unwrap();
    )+}; }
    fields!(
        base,
        "base",
        max_input_bytes,
        max_source_bytes,
        max_include_depth,
        max_include_files,
        max_ast_nesting_depth,
        max_ast_nodes,
        max_style_rules,
        max_text_bytes,
        max_text_buffer_bytes,
        max_shaping_context_bytes,
        max_font_bytes,
        max_fonts,
        max_image_bytes,
        max_images,
        max_resource_bytes,
        max_image_pixels,
        max_decoded_image_bytes,
        max_document_package_bytes,
        max_json_nesting_depth,
        max_pages,
        max_layout_passes,
        max_uri_bytes,
        max_line_reshape_passes,
        max_page_break_lookback,
        max_footnote_reflows_per_page,
        max_column_balance_candidates,
        max_float_queue,
        max_float_carry_pages,
        max_cids_per_font,
        max_fragments,
        max_spool_bytes,
        max_pdf_objects,
        max_output_bytes
    );
    fields!(
        extension,
        "extension",
        max_cff_charstring_operations,
        max_cff_outline_segments,
        max_cff_subroutines,
        max_font_glyphs,
        max_font_subset_bytes,
        max_font_tables,
        max_vector_nodes,
        max_vector_path_segments,
        max_vector_nesting_depth,
        max_math_layout_units
    );
    let limits =
        M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(base).unwrap(), extension)
            .unwrap();
    assert_eq!(limits.canonical_jcs(), canonical);
    limits
}

#[test]
#[ignore = "requires an explicit frozen original-navigation canonical corpus and its hash-bound resource files"]
fn book_v2_column_pdf_attempts_every_original_navigation_package_on_the_public_driver() {
    let path = std::path::PathBuf::from(
        std::env::var_os("TYPAXIS_COLUMN_PDF_ORIGINAL_NAV_CORPUS").unwrap(),
    );
    let raw = fs::read(&path).unwrap();
    let manifest: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(
        manifest["schema"],
        "typaxis.column-pdf-original-navigation-corpus/1"
    );
    let entries = manifest["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 89);
    let mut results = Vec::new();
    let mut successes = 0;
    for entry in entries {
        let frozen = fs::read(entry["original_probe"].as_str().unwrap()).unwrap();
        assert_eq!(
            probes::hex(&typaxis_core::sha256(&frozen)),
            entry["original_sha256"]
        );
        let original: Value = serde_json::from_slice(&frozen).unwrap();
        let canonical = entry["source_canonical_jcs"].as_str().unwrap();
        assert_eq!(
            original["relation_probe"]["marked_probe"]["source_package"]["canonical_jcs"],
            canonical
        );
        assert_eq!(
            probes::hex(&typaxis_core::sha256(canonical.as_bytes())),
            entry["source_sha256"]
        );
        let data: Value = serde_json::from_str(canonical).unwrap();
        let caps = original_corpus_limits(entry["limits_canonical_jcs"].as_str().unwrap());
        let root = Root::new();
        let text = data["text_buffers"][0]["utf8"].as_str().unwrap().as_bytes();
        let body = body_with_source(&root, data.clone(), text, &caps);
        for resource in entry["resources"].as_array().unwrap() {
            let relative = std::path::Path::new(resource["uri"].as_str().unwrap());
            assert!(relative
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))));
            let bytes = fs::read(resource["file"].as_str().unwrap()).unwrap();
            assert_eq!(
                probes::hex(&typaxis_core::sha256(&bytes)),
                resource["sha256"]
            );
            let target = root.0.join(relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, bytes).unwrap();
        }
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config_with_extension(caps.base().get().clone(), caps.extension().get().clone()),
            &caps,
        )
        .unwrap();
        let prepared_canonical =
            typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
                .encode(input.body().styled().body().wire())
                .unwrap();
        assert_eq!(
            prepared_canonical, canonical,
            "{} source changed",
            entry["case"]
        );
        let maximum = entry["maximum_work"].as_u64().unwrap();
        let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
        let mut callback_fact = Value::Null;
        let result = pdf_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, builder, observed| {
                assert_eq!(
                    maximum, MAXIMUM,
                    "the only short original Work case cannot finish an additional PDF stage"
                );
                let display = bundle.display();
                let pages = display.source().source().geometry().pages();
                let names: Vec<_> = pages
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                    .collect();
                let (_, local) = original_regions(
                    &input,
                    &caps,
                    display
                        .source()
                        .original_graph()
                        .lines()
                        .prepared()
                        .shaped()
                        .binding_epoch(),
                    &names,
                );
                callback_fact = json!({"pages":pages.len(),"pdf_bytes":bundle.pdf().bytes().len(),"pdf_sha256":probes::hex(&typaxis_core::sha256(bundle.pdf().bytes())),"navigation_fingerprint":probes::hex(&bundle.navigation().fingerprint())});
                pdf_probes::emit(
                    &format!("pdf-original-{}", entry["case"].as_str().unwrap()),
                    bundle,
                    &caps,
                    observed,
                    &local,
                    builder.reservation_sizes(),
                )
            },
        );
        let error = match result {
            Ok(pending) => {
                successes += 1;
                pdf_probes::finish(pending, budget.observation());
                None
            }
            Err(error) => Some(format!("{error:?}")),
        };
        results.push(json!({"case":entry["case"],"original_scope":entry["original_scope"],"original_error":entry["original_error"],"source_sha256":entry["source_sha256"],"canonical_source_unchanged":true,"error":error,"result":callback_fact,"caller_after":pdf_quartet(budget.observation())}));
    }
    assert!(successes > 0);
    let output = std::path::PathBuf::from(
        std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE").unwrap(),
    )
    .join("column-pdf-corpus");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("original-navigation-package-attempts.json"),serde_json::to_vec_pretty(&json!({"schema":"typaxis.column-pdf-original-navigation-attempts/1","manifest_sha256":probes::hex(&typaxis_core::sha256(&raw)),"attempted":entries.len(),"successful_pdf":successes,"scope":"fresh public PDF driver for each original canonical source and original limits; does not reproduce manual caller/context kernels","results":results})).unwrap()).unwrap();
}

#[derive(Debug)]
struct PdfAttempt {
    constructor: [u64; 4],
    after: [u64; 4],
    error: Option<PdfError>,
    fingerprint: Option<[u8; 32]>,
}

#[test]
fn book_v2_column_pdf_public_driver_four_inclusive_caps_and_object_limit_preserve_failed_prefixes()
{
    let caps = display_caps();
    let data = region_data(2, true, false);
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let (peak, objects, pdf_bytes) =
        pdf_drive(&input, &caps, MODE, &mut warm, 4, |bundle, _, observed| {
            let peak = bundle
                .constructor_charges()
                .iter()
                .map(|q| q[2])
                .chain([observed.output_charge()])
                .max()
                .unwrap();
            (
                peak,
                bundle.pdf().next_object() - 1,
                bundle.pdf().bytes().to_vec(),
            )
        })
        .unwrap();
    let mut totals = pdf_quartet(warm.observation());
    totals[2] = peak;
    for axis in 0..4 {
        for short in [false, true] {
            let cap = totals[axis] - u64::from(short);
            let limited = match axis {
                0 => changed_caps(&caps, |c| c.max_fragments = cap),
                1 => {
                    let mut base = caps.base().get().clone();
                    base.max_spool_bytes = cap;
                    let mut extension = caps.extension().get().clone();
                    extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(cap);
                    M4EffectiveResourceLimits::new(
                        ValidatedResourceLimits::new(base).unwrap(),
                        extension,
                    )
                    .unwrap()
                }
                2 => changed_caps(&caps, |c| c.max_output_bytes = cap),
                _ => caps.clone(),
            };
            let maximum = if axis == 3 { cap } else { MAXIMUM };
            let limited_root = Root::new();
            let limited_input = display_input(&limited_root, data.clone(), None, &limited);
            let mut budget = BookV2ColumnPageBudget::new(&limited, maximum);
            let mut delivered = false;
            let result = pdf_drive(
                &limited_input,
                &limited,
                MODE,
                &mut budget,
                4,
                |bundle, _, _| {
                    delivered = true;
                    assert_eq!(bundle.pdf().bytes(), pdf_bytes);
                    if axis == 2 {
                        let peak = bundle
                            .constructor_charges()
                            .iter()
                            .map(|q| q[2])
                            .chain([bundle.pdf().output_charge()])
                            .max()
                            .unwrap();
                        assert_eq!(peak, cap);
                    }
                },
            );
            if short {
                assert!(result.is_err(), "axis{axis}");
                assert!(!delivered);
                let before = budget.observation();
                assert!(pdf_drive(
                    &limited_input,
                    &limited,
                    MODE,
                    &mut budget,
                    4,
                    |_, _, _| panic!("one-short retry delivered PDF")
                )
                .is_err());
                for retained in [0, 1, 3] {
                    assert!(
                        pdf_quartet(budget.observation())[retained]
                            >= pdf_quartet(before)[retained]
                    );
                }
                assert!(budget.observation().line_reshape_passes() >= before.line_reshape_passes());
            } else {
                assert!(result.is_ok(), "axis{axis}: {result:?}");
                assert!(delivered);
                if axis != 2 {
                    assert_eq!(pdf_quartet(budget.observation())[axis], cap);
                }
            }
            assert!(pdf_quartet(budget.observation())[axis] <= cap);
        }
    }
    for short in [false, true] {
        let limited = changed_caps(&caps, |c| c.max_pdf_objects = objects - u32::from(short));
        let limited_root = Root::new();
        let limited_input = display_input(&limited_root, data.clone(), None, &limited);
        let mut budget = BookV2ColumnPageBudget::new(&limited, MAXIMUM);
        let mut delivered = false;
        let result = pdf_drive(
            &limited_input,
            &limited,
            MODE,
            &mut budget,
            4,
            |bundle, _, _| {
                delivered = true;
                assert_eq!(bundle.pdf().next_object() - 1, objects);
                assert_eq!(bundle.pdf().bytes(), pdf_bytes);
            },
        );
        if short {
            assert!(!delivered);
            let crate::book_v2_resources::BookV2ConvergenceError::Stage { stage, source } =
                result.err().unwrap()
            else {
                panic!("PDF object limit returned another stage kind");
            };
            assert_eq!(stage, "column PDF assembly");
            assert_eq!(
                source.downcast_ref::<PdfError>(),
                Some(&PdfError::Resource(PdfResourceError::Objects))
            );
        } else {
            assert!(result.is_ok());
            assert!(delivered);
        }
    }
}

#[test]
fn book_v2_column_pdf_exact_caps_counted_constructors_and_late_failures_keep_actual_prefixes() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(3, true, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    pdf_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
        let source = bundle.navigation();
        let floor = [
            source.record_charge(),
            source.spool_charge(),
            source.output_charge(),
            source.work_steps(),
        ];
        let run = |prior: [u64; 4], maximum: u64| {
            let [mut r, mut s, mut o, mut w] = [u64::MAX; 4];
            let made = ColumnAssembly::new_counted(
                source, &caps, maximum, prior[0], prior[1], prior[2], prior[3], &mut r, &mut s,
                &mut o, &mut w,
            );
            let constructor = [r, s, o, w];
            match made {
                Err(error) => PdfAttempt {
                    constructor,
                    after: constructor,
                    error: Some(error),
                    fingerprint: None,
                },
                Ok(mut builder) => {
                    let built = builder.build();
                    PdfAttempt {
                        constructor,
                        after: [
                            builder.record_charge(),
                            builder.spool_charge(),
                            builder.output_charge(),
                            builder.work_steps(),
                        ],
                        error: built.as_ref().err().copied(),
                        fingerprint: built.ok().map(|pdf| pdf.fingerprint()),
                    }
                }
            }
        };
        let base = run([0; 4], MAXIMUM);
        assert_eq!(base.error, None);
        assert_eq!(base.fingerprint, Some(bundle.pdf().fingerprint()));
        assert_eq!(
            base.constructor,
            [floor[0] + 1, floor[1], floor[2], floor[3]]
        );
        let limits = caps.base().get();
        let maxima = [
            limits.max_fragments,
            limits.max_spool_bytes,
            limits.max_output_bytes,
        ];
        for axis in 0..3 {
            // Output replacement is signed. Compute the prior from the actual
            // net delta, including a decrease, instead of taking a peak.
            let exact_prior = u64::try_from(
                i128::from(maxima[axis]) + i128::from(floor[axis]) - i128::from(base.after[axis]),
            )
            .unwrap();
            assert!(exact_prior >= floor[axis]);
            let mut prior = [0; 4];
            prior[axis] = exact_prior;
            let exact = run(prior, MAXIMUM);
            assert_eq!(exact.error, None, "axis {axis}: {exact:?}");
            assert_eq!(exact.after[axis], maxima[axis]);
            assert_eq!(exact.fingerprint, base.fingerprint);
            prior[axis] += 1;
            let short = run(prior, MAXIMUM);
            let expected = [
                PdfResourceError::Records,
                PdfResourceError::Spool,
                PdfResourceError::Output,
            ][axis];
            assert_eq!(
                short.error,
                Some(PdfError::Resource(expected)),
                "axis {axis}: {short:?}"
            );
            assert!(short.after[axis] <= maxima[axis]);
            for i in [0, 1, 3] {
                assert!(short.after[i] >= short.constructor[i]);
            }
        }
        let exact = run([0; 4], base.after[3]);
        assert_eq!(exact.error, None);
        assert_eq!(exact.after, base.after);
        assert_eq!(exact.fingerprint, base.fingerprint);
        let late = run([0; 4], base.after[3] - 1);
        assert_eq!(late.error, Some(PdfError::Resource(PdfResourceError::Work)));
        assert_eq!(&late.after[..3], &base.after[..3]);
        assert!(late.after[3] <= base.after[3] - 1);
        let constructor_record_refusal = run([maxima[0], 0, 0, 0], MAXIMUM);
        assert_eq!(
            constructor_record_refusal.error,
            Some(PdfError::Resource(PdfResourceError::Records))
        );
        assert_eq!(
            constructor_record_refusal.constructor,
            [maxima[0], floor[1], floor[2], floor[3]]
        );
        let constructor_work_refusal = run([0; 4], floor[3] - 1);
        assert_eq!(
            constructor_work_refusal.error,
            Some(PdfError::Resource(PdfResourceError::Work))
        );
        assert_eq!(
            constructor_work_refusal.constructor,
            [floor[0] + 1, floor[1], floor[2], floor[3]]
        );
        let shifted = run([0, 0, 0, floor[3] + 17], MAXIMUM);
        assert_eq!(shifted.error, None);
        assert_eq!(shifted.after[3], base.after[3] + 17);
        assert_eq!(shifted.fingerprint, base.fingerprint);

        let changed = changed_caps(&caps, |c| c.max_layout_passes -= 1);
        let prior = [floor[0] + 7, floor[1] + 11, floor[2] + 13, floor[3] + 17];
        let [mut r, mut s, mut o, mut w] = [0; 4];
        let error = ColumnAssembly::new_counted(
            source, &changed, MAXIMUM, prior[0], prior[1], prior[2], prior[3], &mut r, &mut s,
            &mut o, &mut w,
        )
        .err()
        .unwrap();
        assert_eq!(error, PdfError::Resource(PdfResourceError::Identity));
        assert_eq!([r, s, o, w], prior);

        // A completed build spends replacement credit once. Rebuilding the same
        // immutable Navigation source accumulates a full new PDF allocation.
        let mut repeated = ColumnAssembly::new(source, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
        let first = repeated.build().unwrap();
        let second = repeated.build().unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert!(second.record_charge() > first.record_charge());
        assert!(second.spool_charge() > first.spool_charge());
        assert!(second.work_steps() > first.work_steps());
        assert_eq!(
            second.output_charge(),
            first.output_charge() + first.bytes().len() as u64
        );

        let mut late_builder =
            ColumnAssembly::new(source, &caps, base.after[3] - 1, 0, 0, 0, 0).unwrap();
        assert_eq!(
            late_builder.build().err(),
            Some(PdfError::Resource(PdfResourceError::Work))
        );
        let accepted = [
            late_builder.record_charge(),
            late_builder.spool_charge(),
            late_builder.output_charge(),
            late_builder.work_steps(),
        ];
        assert_eq!(&accepted[..3], &base.after[..3]);
        assert_eq!(
            late_builder.build().err(),
            Some(PdfError::Resource(PdfResourceError::Work))
        );
        assert_eq!(
            [
                late_builder.record_charge(),
                late_builder.spool_charge(),
                late_builder.output_charge()
            ],
            [accepted[0], accepted[1], accepted[2]]
        );
    })
    .unwrap();
}

#[test]
fn book_v2_column_pdf_whole_driver_callback_build_and_refused_retry_retain_signed_output() {
    let caps = display_caps();
    let data = region_data(2, true, false);
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut accepted = [0; 4];
    let refusal: Result<(), &str> = pdf_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            assert_eq!(
                pdf_quartet(observed),
                [
                    bundle.pdf().record_charge(),
                    bundle.pdf().spool_charge(),
                    bundle.pdf().output_charge(),
                    bundle.pdf().work_steps()
                ]
            );
            let again = builder.build().unwrap();
            assert_eq!(again.bytes(), bundle.pdf().bytes());
            assert_eq!(
                again.output_charge(),
                bundle.pdf().output_charge() + again.bytes().len() as u64
            );
            accepted = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            Err("column PDF callback refusal")
        },
    )
    .unwrap();
    assert_eq!(refusal, Err("column PDF callback refusal"));
    assert_eq!(pdf_quartet(budget.observation()), accepted);
    let previous_passes = budget.observation().line_reshape_passes();
    pdf_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            assert_eq!(bundle.caller_before(), accepted);
            assert_eq!(pdf_quartet(bundle.previous_observation()), accepted);
            assert!(observed.record_charge() > accepted[0]);
            assert!(observed.spool_charge() > accepted[1]);
            assert!(observed.work_steps() > accepted[3]);
            let nav = bundle.navigation();
            let marked = nav.source().source().source();
            let credit = marked.text().source().bytes().len()
                + marked.source().source().source().bytes().len()
                + marked.byte_length();
            assert_eq!(
                observed.output_charge(),
                nav.output_charge() - credit as u64 + bundle.pdf().bytes().len() as u64
            );
        },
    )
    .unwrap();
    assert!(budget.observation().line_reshape_passes() > previous_passes);

    // The first build succeeds; a callback rebuild then fails after accepting
    // reservations. The caller must receive that builder's final signed O.
    let maximum = accepted[3] - 1;
    let mut short = BookV2ColumnPageBudget::new(&caps, maximum);
    let mut callback_after = [0; 4];
    pdf_drive(&input, &caps, MODE, &mut short, 4, |_, builder, _| {
        assert_eq!(
            builder.build().err(),
            Some(PdfError::Resource(PdfResourceError::Work))
        );
        callback_after = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.output_charge(),
            builder.work_steps(),
        ];
    })
    .unwrap();
    assert_eq!(pdf_quartet(short.observation()), callback_after);
    assert!(callback_after[3] <= maximum);
    let before_retry = short.observation();
    assert!(
        pdf_drive(&input, &caps, MODE, &mut short, 4, |_, _, _| panic!(
            "exhausted retry delivered PDF"
        ))
        .is_err()
    );
    let after_retry = short.observation();
    for axis in [0, 1, 3] {
        assert!(pdf_quartet(after_retry)[axis] >= pdf_quartet(before_retry)[axis]);
    }
    assert!(after_retry.line_reshape_passes() >= before_retry.line_reshape_passes());
}

#[test]
fn book_v2_column_pdf_authored_columns_and_running_regions_emit_real_pdf_with_exact_net_output() {
    for columns in [2, 3] {
        for blank in [false, true] {
            for empty in [false, true] {
                let caps = display_caps();
                let root = Root::new();
                let input = display_input(&root, region_data(columns, blank, empty), None, &caps);
                let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
                let before = budget.observation();
                let (bytes, final_prefix, pending) = pdf_drive(
                    &input,
                    &caps,
                    MODE,
                    &mut budget,
                    1,
                    |bundle, builder, observed| {
                        let pdf = bundle.pdf();
                        let navigation = bundle.navigation();
                        assert!(std::ptr::eq(pdf.navigation(), navigation));
                        assert!(std::ptr::eq(builder.navigation(), navigation));
                        assert_eq!(
                            pdf_quartet(bundle.previous_observation()),
                            pdf_quartet(before)
                        );
                        assert_eq!(bundle.caller_before(), pdf_quartet(before));
                        let constructors = bundle.constructor_charges();
                        assert_eq!(
                            &constructors[..13],
                            &bundle.navigation_bundle().constructor_charges()
                        );
                        let prefix = bundle.navigation_prefix();
                        assert_eq!(
                            constructors[13],
                            [
                                prefix.record_charge() + 1,
                                prefix.spool_charge(),
                                prefix.output_charge(),
                                prefix.work_steps()
                            ]
                        );
                        let display = bundle.display();
                        let pages = display.source().source().geometry().pages();
                        assert!(!pages.is_empty());
                        assert_eq!(display.page_regions().len(), pages.len() * 2);
                        for (index, page) in pages.iter().enumerate() {
                            assert_eq!(page.columns().len(), columns as usize);
                            assert!(pdf.page_object(index).is_some());
                        }
                        assert!(pdf.page_object(pages.len()).is_none());
                        assert!(pdf.page_reference_labels_match());
                        assert!(pdf.page_references().is_empty());
                        let marked = navigation.source().source().source();
                        let font_objects = marked.text().source();
                        let image_objects = marked.source().source().source();
                        let credit = font_objects.bytes().len()
                            + image_objects.bytes().len()
                            + marked.byte_length();
                        let expected_output = navigation
                            .output_charge()
                            .checked_sub(credit as u64)
                            .unwrap()
                            + pdf.bytes().len() as u64;
                        assert_eq!(pdf.output_charge(), expected_output);
                        assert_eq!(observed.output_charge(), expected_output);
                        assert_eq!(
                            pdf_quartet(observed),
                            [
                                builder.record_charge(),
                                builder.spool_charge(),
                                builder.output_charge(),
                                builder.work_steps()
                            ]
                        );
                        assert!(pdf.bytes().starts_with(b"%PDF-1.7\n"));
                        assert!(pdf.bytes().ends_with(b"%%EOF\n"));
                        assert!(!pdf.object_bytes(pdf.catalog_object()).unwrap().is_empty());
                        let bytes = pdf.bytes().to_vec();
                        let names = pages
                            .iter()
                            .enumerate()
                            .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                            .collect::<Vec<_>>();
                        let epoch = display
                            .source()
                            .original_graph()
                            .lines()
                            .prepared()
                            .shaped()
                            .binding_epoch();
                        let (_, local) = original_regions(&input, &caps, epoch, &names);
                        let pending = pdf_probes::emit(
                            &format!("column-pdf-regions-{columns}-{blank}-{empty}"),
                            bundle,
                            &caps,
                            observed,
                            &local,
                            builder.reservation_sizes(),
                        );
                        (bytes, observed, pending)
                    },
                )
                .unwrap();
                pdf_probes::finish(pending, budget.observation());
                assert!(!bytes.is_empty());
                assert_eq!(pdf_quartet(budget.observation()), pdf_quartet(final_prefix));
            }
        }
    }
}
